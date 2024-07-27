# Step 1 - Setting the O_NONBLOCK Flag

The first step to asynchronous I/O is to change the I/O handle to be nonblocking by setting the `O_NONBLOCK` flag.

### API

`Async::new(handle)` is responsible for setting the I/O handle to be nonblocking.

### Implementation

`Async::new(handle)` is the constructor of the `Async` struct. For example, here is how you create an instance of `Async<TcpListener>`:

```rust
let listener = TcpListener::bind(addr)?;
Async::new(listener);
```

Here is the implementation of `Async::new`:

```rust
impl<T: AsRawFd> Async<T> {
    pub fn new(io: T) -> io::Result<Async<T>> {
        Ok(Async {
            source: get_reactor().create_source(io.as_raw_fd()),
            io: Some(Box::new(io)),
        })
    }
}
```

Note the importance of the `T: AsRawFd` trait. The reactor needs the raw file descriptor of the I/O source in order to check when the I/O operation
is complete with the help of io_uring.

The `get_reactor()` method retrieves the `Reactor` for the executor running on the current thread. The `create_source` method, as shown below, sets the `O_NONBLOCK` flag for the handle with [fcntl](https://man7.org/linux/man-pages/man2/fcntl.2.html).

```rust
impl Reactor {
	...
	pub fn create_source(&self, raw: RawFd) -> Source {
      fcntl(raw, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();
      self.new_source(raw, SourceType::PollableFd)
  }

	fn new_source(&self, raw: RawFd, stype: SourceType) -> Source {
        Source::new(raw, stype, None)
    }

}
```
