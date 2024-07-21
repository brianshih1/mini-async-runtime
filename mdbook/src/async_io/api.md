# API

Our goal here is to implement a set of internal APIs to make it easy to convert synchronous I/O operations into asynchronous ones.

Here are the rough steps to convert a blocking I/O operation into an asynchronous one:

- we set the file descriptor to non-blocking
- we perform the non-blocking operation
- we tell `io_uring` to monitor the file descriptor by submitting an `SQE`
-  we store the poller’s `waker` and invoke `wake()` when the I/O operation is complete. We detect when an I/O operation is complete when the corresponding `CQE` is posted to the `io_uring`'s completion queue.

To make it easier to implement new asynchronous operations, we introduce `Async`, an adapter for I/O types inspired by the [async_io crate](https://docs.rs/async-io/latest/async_io/). `Async` abstracts away the steps listed above so that developers who build on top of `Async` don’t have to worry about things like `io_uring`, `Waker`, `O_NONBLOCK`, etc.

Here is how you use the `Async` adapter to implement an asynchronous `TcpListener` with an asynchronous `accept` method:

```rust
impl Async<TcpListener> {
    pub fn bind<A: Into<SocketAddr>>(addr: A) -> io::Result<Async<TcpListener>> {
        let addr = addr.into();
        let listener = TcpListener::bind(addr)?;
        Ok(Async::new(listener)?)
    }

    pub async fn accept(&self) -> io::Result<(Async<TcpStream>, SocketAddr)> {
        let (stream, addr) = self.read_with(|io| io.accept()).await?;
        Ok((Async::new(stream)?, addr))
    }
}
```

Here is how you can use the `Async<TcpListener>` inside an executor to perform asynchronous I/O:

```rust
let local_ex = LocalExecutor::default();
let res = local_ex.run(async {
    let listener = Async::<TcpListener>::bind(([127, 0, 0, 1], 8080)).unwrap();
    let (stream, _) = listener.accept().await.unwrap();
    handle_connection(stream);
});
```

Next, let's look at what the `Async` adapter actually does.