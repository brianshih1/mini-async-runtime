# TaskQueue

An `executor` needs to store a list of scheduled `Task`s. This is what the `TaskQueue` is for, it holds a collection of managed tasks.

Here is the implemnetation for the `TaskQueue`:

```rust
pub(crate) struct TaskQueue {
    // contains the actual queue of Tasks
    pub(crate) ex: Rc<TaskQueueExecutor>,
    // The invariant around active is that when it's true,
    // it needs to be inside the active_executors
    pub(crate) active: bool,
}

pub(crate) struct TaskQueueExecutor {
    local_queue: LocalQueue,
    name: String,
}

struct LocalQueue {
    queue: RefCell<VecDeque<Task>>,
}
```

The `TaskQueue` contains a `TaskQueueExecutor` which contains the actual `LocalQueue` which holds a `VecDeque` of `Task`s.

The two most important methods on a `TaskQueueExecutor` are:

- create_task
- spawn_and_schedule

**create_task**

Create task allocates the `Task` and creates the corresponding `JoinHandle`. Note that creating a `Task` requires providing a `schedule` method. The provided `schedule` method is a closure that simply pushes the `task` onto the `local_queue`.

```rust
// Creates a Task with the Future and push it onto the queue by scheduling
fn create_task<T>(
    &self,
    executor_id: usize,
    tq: Rc<RefCell<TaskQueue>>,
    future: impl Future<Output = T>,
) -> (Task, JoinHandle<T>) {
    let tq = Rc::downgrade(&tq);
    let schedule = move |task| {
        let tq = tq.upgrade();

        if let Some(tq) = tq {
            {
                tq.borrow().ex.as_ref().local_queue.push(task);
            }
            {
                LOCAL_EX.with(|local_ex| {
                    let mut queues = local_ex.queues.as_ref().borrow_mut();
                    queues.maybe_activate_queue(tq);
                });
            }
        }
    };
    create_task(executor_id, future, schedule)
}
```

**spawn_and_schedule**

```rust
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

`spawn_and_schedule` simply creates the task and invokes the `schedule` method which pushes the `task` onto the `LocalQueue` of the `TaskQueueExecutor`.

### Code References

To check out my toy implementation or Glommio’s implementation, check out:

**My Toy Implementation**

- [TaskQueue](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/task_queue.rs#L16)
- [TaskQueueExecutor::create_task](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/task_queue.rs#L79)
- [TaskQueueExecutor::spawn_and_schedule](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/task_queue.rs#L110)

**Glommio**

- [TaskQueue](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/executor/mod.rs#L126)
- [LocalExecutor](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/executor/multitask.rs#L114) - My toy implementation calls the `LocalExecutor` the `TaskQueueExecutor`
