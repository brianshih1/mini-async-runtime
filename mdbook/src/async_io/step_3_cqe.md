# Step 3 - Processing the CQE

After adding a `SQE` to the `io_uring`'s submission queue, the executor needs a way to detect when the I/O operation is completed and resume the task that is blocked.

Detecting when the I/O operation is completed is done by checking if there are new `CQE` entries on the `io_uring` instance’s completion queue. Resuming the task that is blocked is performed by calling `wake()` on the stored `Waker` in the `Source`.

### API

Each `Reactor` has a `wait` API that the executor can use to check for new CQE entries and process the completed event. Here is its API:

```rust
pub(crate) fn wait(&self) -> ()
```

### Implementation

The `Reactor::wait` API first calls `consume_completion_queue` to check if there are any new `CQE` entries. It then calls `consume_submission_queue` to submit `SQE` entries to the kernel as covered in the last page.

```rust
impl Reactor {
		...
		
		pub(crate) fn wait(&self) {
        let mut main_ring = self.main_ring.borrow_mut();
        main_ring.consume_completion_queue();
        main_ring.consume_submission_queue().unwrap();
    }
}
```

Here is the implementation of `consume_completion_queue`. It simply calls `consume_one_event` repeatedly until there are no more new `CQE` events. `Consume_one_event` simply invokes `process_one_event`.

```rust
pub(crate) trait UringCommon {
		...

		fn consume_completion_queue(&mut self) -> usize {
        let mut completed: usize = 0;
        loop {
            if self.consume_one_event().is_none() {
                break;
            } else {
            }
            completed += 1;
        }
        completed
    }
}

impl UringCommon for SleepableRing {
	fn consume_one_event(&mut self) -> Option<bool> {
      let source_map = self.source_map.clone();
      process_one_event(self.ring.peek_for_cqe(), source_map).map(|x| {
          self.in_kernel -= 1;
          x
      })
  }
}
```

Here is the implementation for `process_one_event`:

```rust
fn process_one_event(cqe: Option<iou::CQE>, source_map: Rc<RefCell<SourceMap>>) -> Option<bool> {
    if let Some(value) = cqe {
        // No user data is `POLL_REMOVE` or `CANCEL`, we won't process.
        if value.user_data() == 0 {
            return Some(false);
        }

        let src = source_map.borrow_mut().consume_source(value.user_data());

        let result = value.result();

        let mut woke = false;

        let mut inner_source = src.borrow_mut();
        inner_source.wakers.result = Some(result.map(|v| v as usize));
        woke = inner_source.wakers.wake_waiters();

        return Some(woke);
    }
    None
}
```

The method first retrieves the `Source` with the `user_data` on the `CQE`. Next, it wakes up the waiters stored on the `Source`. This resumes the tasks blocked by scheduling them back onto the executor.

The executor calls `Reactor::wait` on each iteration in the `loop` inside the `run` method via the `poll_io` method as shown below:

```rust
/// Runs the executor until the given future completes.
pub fn run<T>(&self, future: impl Future<Output = T>) -> T {
		...
    LOCAL_EX.set(self, || {
        ...
        loop {
            if let Poll::Ready(t) = join_handle.as_mut().poll(cx) {
                ...
            }
            // this is what processes the completed I/O events (from the completion queue)
            // and reschedules any blocked tasks.
            get_reactor().react();
            ...
        }
    })
```
