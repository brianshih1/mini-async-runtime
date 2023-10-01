# Waker

Earlier, we saw that when `RawTask::run` is called, the method creates a `waker` when `poll`ing the user-provided `Future`:

```rust
let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE)));
let cx = &mut Context::from_waker(&waker);

let poll = <F as Future>::poll(Pin::new_unchecked(&mut *raw.future), cx);
```

So what does the `Waker` need to do? When `Waker::wake()` is called, the `Waker` needs to notify the executor that the `Task` is ready to be `run` again. Therefore, `Waker::wake()` needs to `schedule` the `Task` by pushing it back to the `TaskQueue`. Let’s look at how we can add a `Task` back to the `TaskQueue` when `Waker::wake` is called.

To create a `Waker`, you need to pass it a `RAW_WAKER_VTABLE`. The `Waker` is created with `Waker::from_raw(RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE))`. The `RAW_WAKER_VTABLE` is just a virtual function pointer table to methods like `wake`.

When `Waker::wake()` is called, the actual wakeup call is delegated through a virtual function call to the implementation which is defined by the executor.

`RAW_WAKER_VTABLE` is defined as a `constant` variable in the `RawTask`:

```rust
impl<F, R, S> RawTask<F, R, S>
where
    F: Future<Output = R>,
    S: Fn(Task),
{
    const RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_waker,
        Self::wake,
        Self::wake_by_ref,
        Self::drop_waker,
    );
```

Here is the implementation of `wake` and `wake_by_ref`:

```rust
unsafe fn wake(ptr: *const ()) {
    Self::wake_by_ref(ptr);
    Self::drop_waker(ptr);
}

/// Wakes a waker. Ptr is the raw task.
unsafe fn wake_by_ref(ptr: *const ()) {
    let raw = Self::from_ptr(ptr);
	  let state = (*raw.header).state;
	
	  // If the task is completed or closed, it can't be woken up.
	  if state & (COMPLETED | CLOSED) == 0 {
	      // If the task is already scheduled do nothing.
	      if state & SCHEDULED == 0 {
	          // Mark the task as scheduled.
	          (*(raw.header as *mut Header)).state = state | SCHEDULED;
	          if state & RUNNING == 0 {
	              // Schedule the task.
	              Self::schedule(ptr);
	          }
	      }
	  }
}
```

`Wake` simply calls `Self::schedule` if the task is not completed, not closed, and not scheduled.

`RawTask::schedule` simply calls `raw.schedule`, which is a property on the `RawTask` provided by the executor during the creation of the `RawTask`.

```rust
unsafe fn schedule(ptr: *const ()) {
    let raw = Self::from_ptr(ptr);

		...
    let task = Task {
        raw_task: NonNull::new_unchecked(ptr as *mut ()),
    };

    (*raw.schedule)(task);
}
```

In `create_task` below, we can see that the executor provides a `schedule` callback that simply pushes the task back onto the `local_queue`.

```rust
fn create_task<T>(
    &self,
    executor_id: usize,
    tq: Rc<RefCell<TaskQueue>>,
    future: impl Future<Output = T>,
) -> (Task, JoinHandle<T>) {
    ...
    let schedule = move |task| {
	      ...
        if let Some(tq) = tq {
          tq.borrow().ex.as_ref().local_queue.push(task);
          ...
        }
    };
    create_task(executor_id, future, schedule)
}
```

### Code References

To check out my toy implementation or Glommio’s implementation, check out:

**Mini Async Runtime**

- [wake_by_ref](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L168)
- [RawTask::schedule](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L192)
- [TaskQueueExecutor::create_task](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/task_queue.rs#L80)

**Glommio**

- [wake_by_ref](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/raw.rs#L259)
- [RawTask::schedule](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/raw.rs#L363)
