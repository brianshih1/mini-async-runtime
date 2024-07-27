# Waker

Earlier, we saw that when `RawTask::run` is called, `run` creates a `waker` which is used to `poll` the user-provided `Future`. In this section, we look at how the `Waker` instance is created.

To create a `waker` in Rust, we need to pass a `RawWakerVTable` to the `Waker` constructor.

Here is the vtable for the task:
```
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

The most important method here is the `wake` method, which is invoked when `Waker::wake` is called.

The `Waker::wake()` simply reschedules the task by pushing it onto the `TaskQueue`. Here is the implementation of `wake` and `wake_by_ref`:

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

The schedule method is passed to the `task` when the task is created and it looks something like:
```
let schedule = move |task| {
    let task_queue = tq.upgrade();
    task_queue.local_queue.push(task);
};
create_task(executor_id, future, schedule)
```

Finally, here is the code that actually creates the `waker` which is used to poll the user-defined future.

```rust
let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE)));
let cx = &mut Context::from_waker(&waker);

let poll = <F as Future>::poll(Pin::new_unchecked(&mut *raw.future), cx);
```

### Code References

To check out my toy implementation or Glommio’s implementation, check out:

**My Toy Implementation**

- [wake_by_ref](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L168)
- [RawTask::schedule](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L192)
- [TaskQueueExecutor::create_task](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/task_queue.rs#L80)

**Glommio**

- [wake_by_ref](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/raw.rs#L259)
- [RawTask::schedule](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/raw.rs#L363)
