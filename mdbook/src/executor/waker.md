# Waker

When the executor polls a future, it returns `Poll::Pending` if it's blocked by another operation, i.e. waiting for the kernel to finish
reading from disk. The question is when should the executor poll again?

A dumb solution is to have the executor periodically poll the `Future` to check if it's ready yet. But that’s inefficient and wastes CPU cycles.

Instead, a more efficient solution is to pass a callback to the `Future` and have the `Future` invoke the callback when it is unblocked. This is what the `Waker` is for.

The `Waker` is passed to the `Future` each time it's `poll`ed. As a refresher, here is the function signature for `poll`:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
```

Each `Context` struct contains a `waker` that can be retrieved with `cx.waker()`. Each `waker` has a `wake()` method, which notifies the executor that the `Future` is ready to be `poll`ed again.

To create a `Waker`, we can use the `from_raw` constructor:

```rust
pub const unsafe fn from_raw(waker: RawWaker) -> Waker
```

Each `waker` has a `wake` method that can be called when it is done.

Here is an example borrowed from the [async book](https://rust-lang.github.io/async-book/02_execution/03_wakeups.html)
that implements a timer with the waker:

```
pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

impl Future for TimerFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            Poll::Ready(())
        } else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl TimerFuture {
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        // Spawn the new thread
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.completed = true;
            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        });

        TimerFuture { shared_state }
    }
}
```

In this example, a thread is created when the `TimerFuture` is created. The thread simply performs `thread::sleep` which acts as the timer.
When the future is polled, the `waker` is stored. When the `thread::sleep` completes, the `waker::wake` method is called to notify the poller that it can be polled again.