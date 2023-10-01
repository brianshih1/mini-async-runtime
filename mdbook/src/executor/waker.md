# Waker

When `poll`ed, a `Future` returns `Poll::Pending` if it’s blocked by another operation (e.g. waiting for a disk read to complete). The executor can keep polling the `Future` at regular intervals to check if it’s ready yet. But that’s inefficient and wastes CPU cycles.

A more efficient solution is to poll again when the operation blocking the `Future` from making progress is finished (e.g. when the disk read is complete). Ideally, the blocking operation notifies the executor that the `Future` is ready to be `poll`ed again. This is what the `Waker` is for.

The `poll` method of the `Future` contains a `Context`:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
```

Each `Context` has a waker that can be retrieved with `cx.waker()`. Each `waker` has a `wake()` method, which notifies the executor that the `Future` is ready to be `poll`ed again.

So what exactly is a `Waker`? The `Waker` is a `struct` that contains function pointers. However, what the functions do is totally up to the executor that polls the `Future`.

To create a `Waker`, we can use the `from_raw` constructor:

```rust
pub const unsafe fn from_raw(waker: RawWaker) -> Waker
```

The `RawWaker` contains a `RawWakerVTable` which contains pointers to methods like `wake`.

```rust
pub struct RawWaker {
		...
    /// Virtual function pointer table that customizes the behavior of this waker.
    vtable: &'static RawWakerVTable,
}

pub struct RawWakerVTable {
    clone: unsafe fn(*const ()) -> RawWaker,
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
    drop: unsafe fn(*const ()),
}
```

When `Waker::wake` is called, the `RawWakerVTable`'s `wake` method is called. Below is the implementation of `Waker::wake()`. We can see that it simply calls the virtual function implemented by the executor.

```rust
pub fn wake(self) {
  // The actual wakeup call is delegated through a virtual function call
  // to the implementation which is defined by the executor.
  let wake = self.waker.vtable.wake;
  let data = self.waker.data;

  // Don't call `drop` -- the waker will be consumed by `wake`.
  crate::mem::forget(self);

  // SAFETY: This is safe because `Waker::from_raw` is the only way
  // to initialize `wake` and `data` requiring the user to acknowledge
  // that the contract of `RawWaker` is upheld.
  unsafe { (wake)(data) };
}
```

A common pattern is that the `wake` method adds the `Future` back to a queue. The executor can then loop over the queue of `Future`s that are ready to be `poll`ed again.

Let’s look at an example:

```rust
async fn send_email() {
    async_send_email(...).await;
}
```

In this example, when `send_email` is `poll`ed, it returns `Poll::pending` because making a network request to send the email takes time. However, the `cx.waker()` is stored by the email client. When the email client finishes sending the email, it calls `waker.wake()` to notify the executor that the `Future` is ready to be `poll`ed again.
