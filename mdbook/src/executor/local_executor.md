# LocalExecutor

Now that we know how the task and task queue is implemented, we can perform a deep dive 
into how the `Executor::Run` and `Executor::spawn` methods are implemented.

As a refresher, here’s an example of an executor running a task and spawning a task onto the executor.

```rust
let local_ex = LocalExecutor::default();
let res = local_ex.run(async {
    let handle = local_ex.spawn(async_write_file());
    handle.await;
});
```

### Single Threaded

It’s important to understand that `LocalExecutor` is a single-threaded executor. This means that the executor can only be run on the thread that created it. `LocalExecutor` doesn’t implement the `Send` or `Sync` trait, so you cannot move a `LocalExecutor` across threads. This makes it easier to reason about the methods on `LocalExecutor` since it’s safe to assume that only one function invocation can be executing at any time. In other words, there won’t be two invocations of `run` on the same executor at once.

Conceptually, the way to think about an executor is that it stores a collection of Task Queues. Each Task Queue has a collection of Tasks to execute. When `run` is called, the executor would choose one of the task queues to be the active executor. Then it will start looping and popping tasks off the Task Queue.

### Internals

Let’s look at the internals of an Executor:

```rust
pub(crate) struct LocalExecutor {
    pub(crate) id: usize,
    pub(crate) queues: Rc<RefCell<QueueManager>>,
}
```

A `LocalExecutor` contains a `QueueManager`. As explained earlier, a `QueueManager` contains all the `Task Queues`.

```rust
pub(crate) struct QueueManager {
    pub active_queues: BinaryHeap<Rc<RefCell<TaskQueue>>>,
    pub active_executing: Option<Rc<RefCell<TaskQueue>>>,
    pub available_queues: AHashMap<usize, Rc<RefCell<TaskQueue>>>,
}
```

At any time, a `QueueManager` is actively working on at most one `TaskQueue`. The `active_queues` property stores the `TaskQueues` that are not empty. Any `TaskQueue` inside `active_queues` is also inside `available_queues`. A `TaskQueue` is removed from `active_queues` whenever it’s empty.

Now, we can finally look at `run`, the core method of a `LocalExecutor`.

### Deep Dive into Run

The `run` method runs the executor until the provided `future` completes. Here is its implementation:

```rust
pub fn run<T>(&self, future: impl Future<Output = T>) -> T {
    assert!(
        !LOCAL_EX.is_set(),
        "There is already an LocalExecutor running on this thread"
    );
    LOCAL_EX.set(self, || {
        let join_handle = self.spawn(async move { future.await });
        let waker = dummy_waker();
        let cx = &mut Context::from_waker(&waker);
        pin!(join_handle);
        loop {
            if let Poll::Ready(t) = join_handle.as_mut().poll(cx) {
                // can't be canceled, and join handle is None only upon
                // cancellation or panic. So in case of panic this just propagates
                return t.unwrap();
            }

            // TODO: I/O work
            self.run_task_queues();
        }
    })
}
```

Let’s break down `run` line by line. First, `run` makes sure that no other executors are running on the same thread. `LOCAL_EX` is a thread local storage key defined as:

```rust
scoped_tls::scoped_thread_local!(static LOCAL_EX: LocalExecutor);
```

Next, it calls `spawn` to create and schedule the task onto the `TaskQueue`.

It then loops until the `future` is completed. It’s super important to understand that the `poll` method here doesn’t actually `poll` the user-provided future. It simply `poll`s the `JoinHandle`, which checks if the `COMPLETED` flag on the task’s `state` is set.

Since the `executor` is single-threaded, looping alone won’t actually progress the underlying future. Therefore, in each loop, the `executor` calls the `run_task_queues` method.

`run_task_queues` simply loops and calls `run_one_task_queue` until there are no more `task`s left in the `TaskQueue`.

```rust
fn run_task_queues(&self) -> bool {
    let mut ran = false;
    loop {
        // TODO: Check if prempt
        if !self.run_one_task_queue() {
            return false;
        } else {
            ran = true;
        }
    }
    ran
}
```

`run_one_task_queue` sets the `active_executing` queue to one of the `active_queues`. It then loops until until there are no more tasks in that `TaskQueue`.

In each loop, it calls `get_task` which pops a `task` from the `TaskQueue`.

```rust
fn run_one_task_queue(&self) -> bool {
  let mut q_manager = self.queues.borrow_mut();
  let size = q_manager.active_queues.len();
  let tq = q_manager.active_queues.pop();
  match tq {
      Some(tq) => {
          q_manager.active_executing = Some(tq.clone());
          drop(q_manager);
          loop {
              // TODO: Break if pre-empted or yielded
              let tq = tq.borrow_mut();

              if let Some(task) = tq.get_task() {
                  drop(tq);
                  task.run();
              } else {
                  break;
              }
          }
          true
      }
      None => {
          false
      }
  }
}
```

To summarize, `run` spawns a task onto one of the task queues. The executor then runs one `task_queue` at a time to completion until the spawned `task` is `COMPLETED`. The most important concept to remember here is that none of the task is `blocking`. Whenever one of the `task` is about to be run, it is popped from the `TaskQueue`. It won’t be scheduled back onto the `TaskQueue` until its `waker` is invoked, which is when the thing blocking it is no longer blocking. In other words, the `executor` will move from one `task` to another without waiting on any blocking code.

### spawn

The `spawn` method is how a user can spawn a task onto the executor.

`spawn` allows the developer to create two tasks that run concurrently instead of sequentially:

```rust
let res = local_ex.run(async {
    let handle1 = local_ex.spawn(async_write_file());
		let handle2 = local_ex.spawn(async_write_file());
    handle1.await;
    handle2.await;
});
```

This is the implementation of `spawn`:

`Spawn_local` simply finds the `LocalExecutor` on the current thread and calls `LocalExecutor::spawn`. Here is the implementation of `spawn`:

```rust
pub(crate) fn spawn<T>(&self, future: impl Future<Output = T>) -> JoinHandle<T> {
    let active_executing = self.queues.borrow().active_executing.clone();
    let tq = active_executing
        .clone() // this clone is cheap because we clone an `Option<Rc<_>>`
        .or_else(|| self.get_default_queue())
        .unwrap();
    let tq_executor = tq.borrow().ex.clone();
    tq_executor.spawn_and_schedule(self.id, tq, future)
}

pub(crate) fn spawn_and_schedule<T>(
    &self,
    executor_id: usize,
    tq: Rc<RefCell<TaskQueue>>,
    future: impl Future<Output = T>,
) -> JoinHandle<T> {
    let (task, handle) = self.create_task(executor_id, tq, future);
    task.schedule();
    handle
}
```

`Spawn` gets the active executing `TaskQueue`, creates a task and schedules the `Task` onto the `TaskQueue`.

To summarize, `spawn_local` simply schedules a `Task` onto the `LocalExecutor`'s actively executing `TaskQueue`.

### Code References

To check out my toy implementation or Glommio’s implementation, check out:

**My Toy Implementation**

- [LocalExecutor::run](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L65)
- [LocalExecutor::spawn](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L55)

**Glommio**

- [LocalExecutor::run](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/executor/mod.rs#L1429)
- [LocalExecutor::spawn](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/executor/mod.rs#L632)
