# Future

A future represents a value that might not be available yet. Each future implements the `std::future::Future` trait as follows:

```rust
pub trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>;
}
```

The associated type, `Output`, represents the type of the output value.

The `poll` method returns whether the value is ready or not. It’s also used to advance the `Future` towards completion. The `poll` method returns `Poll::Ready(value)` if it’s completed and `Poll::Pending` if it’s not complete yet. It’s important to understand that a `Future` does nothing until it’s `poll`ed. Polling a future forces it to make progress.

The `Poll` enum looks like this:

```rust
pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

The `poll` method takes in a `Context` argument. As we will cover soon, the `Context` holds a `Waker` instance which notifies any interested tasks that are blocked by the current task.
