# Running the Task

When the `Task` is run, the task doesn’t just `poll` the user-provided `Future`. It also needs to perform memory accounting and handle edge cases.

Let’s break it down section by section.

```rust
unsafe fn run(ptr: *const ()) -> bool {
		let raw = Self::from_ptr(ptr);
		
		let mut state = (*raw.header).state;
		
		// Update the task's state before polling its future.
		// If the task has already been closed, drop the task reference and return.
		if state & CLOSED != 0 {
		    // Drop the future.
		    Self::drop_future(ptr);
		
		    // Mark the task as unscheduled.
		    (*(raw.header as *mut Header)).state &= !SCHEDULED;
		
		    // Notify the awaiter that the future has been dropped.
		    (*(raw.header as *mut Header)).notify(None);
		
		    // Drop the task reference.
		    Self::drop_task(ptr);
		    return false;
		}
		...
}
```

First, we check if the task is already closed. If it is, we want to return early. But before returning, we need to unset the `SCHEDULED` bit of the Task’s `state`. We also want to notify the awaiter (blocked task) that it is unblocked.

The `notify` method’s implementation is as follows:

```rust
/// Notifies the awaiter blocked on this task.
pub(crate) fn notify(&mut self, current: Option<&Waker>) {
    let waker = self.awaiter.take();

		// TODO: Check against current
    if let Some(w) = waker {
        w.wake()
    }
}
```

As mentioned earlier, a task stores the `waker`. The `notify` method calls the `waker`.

If the `Task` isn’t closed, we can proceed with running the Task. First, we update the `state` of the `Task` by unsetting the `SCHEDULED` bit and setting the `RUNNING` bit.

```rust
// Unset the Scheduled bit and set the Running bit
state = (state & !SCHEDULED) | RUNNING;
(*(raw.header as *mut Header)).state = state;
```

Next, we poll the Task’s Future. Polling a future requires a `waker`. We create one with `RAW_WAKER_VTABLE` which we will cover in more detail in another page.

```rust
let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE)));
let cx = &mut Context::from_waker(&waker);

let poll = <F as Future>::poll(Pin::new_unchecked(&mut *raw.future), cx);
```

If polling the future returns `Poll::Ready`, we need to do some housekeeping:

- since we never need to poll the future again, we can drop it
- We update the state to not be `(state & !RUNNING & !SCHEDULED) | COMPLETED`. If the `HANDLE` is dropped, then we also need to mark it as `CLOSED`. This is because the definition of `CLOSED` is when the output of the `JoinHandle` has been consumed. If the `JoinHandle` is dropped, the output of the `Task` is not needed so it’s technically “consumed”.
- In the case that the output is not needed, which is when the `HANDLE` is dropped or if the task was closed while running, we can drop the `output` early since no one will consume it.

```rust
match poll {
  Poll::Ready(out) => {
      Self::drop_future(ptr);
      raw.output.write(out);

      // A place where the output will be stored in case it needs to be dropped.
      let mut output = None;

      // The task is now completed.
      // If the handle is dropped, we'll need to close it and drop the output.
      // We can drop the output if there is no handle since the handle is the
      // only thing that can retrieve the output from the raw task.
      let new = if state & HANDLE == 0 {
          (state & !RUNNING & !SCHEDULED) | COMPLETED | CLOSED
      } else {
          (state & !RUNNING & !SCHEDULED) | COMPLETED
      };

      (*(raw.header as *mut Header)).state = new;

      // If the handle is dropped or if the task was closed while running,
      // now it's time to drop the output.
      if state & HANDLE == 0 || state & CLOSED != 0 {
          // Read the output.
          output = Some(raw.output.read());
      }

      // Notify the awaiter that the task has been completed.
      (*(raw.header as *mut Header)).notify(None);

      drop(output);
  }
  Poll::Pending => {
			...
	}
}
```

Let’s look at what happens if the future returns `Poll::Pending`. In most cases, all we need to do here is to unset the `RUNNING` bit of the task. However, in the case that the task was closed while running, we need to invoke `drop_future` to deallocate the future. We would also want to notify the `awaiter` if the Task is closed while running.

Note that the task can be closed while running in a few scenarios:

- the JoinHandle is dropped
- JoinHandle::cancel is called
- the task panics while running, which will automatically close the task.

Here is the code when the future returns `Poll::Pending`:

```rust
Poll::Pending => {
		// The task is still not completed.

		// If the task was closed while running, we'll need to unschedule in case it
		// was woken up and then destroy it.
		let new = if state & CLOSED != 0 {
		    state & !RUNNING & !SCHEDULED
		} else {
		    state & !RUNNING
		};
		
		if state & CLOSED != 0 {
		    Self::drop_future(ptr);
		}
		
		(*(raw.header as *mut Header)).state = new;
		
		// If the task was closed while running, we need to notify the awaiter.
		// If the task was woken up while running, we need to schedule it.
		// Otherwise, we just drop the task reference.
		if state & CLOSED != 0 {
		    // Notify the awaiter that the future has been dropped.
		    (*(raw.header as *mut Header)).notify(None);
		} else if state & SCHEDULED != 0 {
		    // The thread that woke the task up didn't reschedule it because
		    // it was running so now it's our responsibility to do so.
		    Self::schedule(ptr);
		    ret = true;
		}
}
```

Finally, `drop_task` is called to potentially deallocate the task:

```rust
Self::drop_task(ptr);
```

Here is the implementation for `drop_task`:

```rust
unsafe fn drop_task(ptr: *const ()) {
  let raw = Self::from_ptr(ptr);

  // Decrement the reference count.
  let refs = Self::decrement_references(&mut *(raw.header as *mut Header));

  let state = (*raw.header).state;

  // If this was the last reference to the task and the `JoinHandle` has been
  // dropped too, then destroy the task.
  if refs == 0 && state & HANDLE == 0 {
      Self::destroy(ptr);
  }
}  
```

Note that `drop_task` only deallocates the `task` if the reference count is `0` and the `HANDLE` is dropped. The `HANDLE` is not part of the reference count.

The goal of this section is to showcase the type of challenges that one can expect when building an asynchronous runtime. One needs to pay particular attention to deallocating memory as early as possible and be careful about updating the state of the Task in different scenarios.

### Code References

To check out my toy implementation or Glommio’s implementation, check out:

**Mini Async Runtime**

- [RawTask::run](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L297)

**Glommio**

- [RawTask::run](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/raw.rs#L432)
