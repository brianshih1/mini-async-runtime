# Nonblocking Mode

In Rust, by default, many I/O operations, such as reading a file, are blocking. For example, in the code snippet below, the `TcpListener::accept` call will block the calling thread until a new TCP connection is established.

```rust
let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
listener.accept();
```

### Nonblocking I/O

The first step towards asynchronous I/O is turning a blocking I/O operation into a non-blocking one.

In Linux, it is possible to do nonblocking I/O on sockets and files by setting the `O_NONBLOCK` flag on the file descriptors.

Here’s how you can set the file descriptor for a socket to be non-blocking:

```rust
let listener = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
let raw_fd = listener.as_raw_fd();
fcntl(raw_fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK))
```

Setting the file descriptor for the `TcpListener` to nonblocking means that the next I/O operation would immediately return. To check if the operation is complete, you have to manually `poll` the file descriptor.

Rust’s std library has helper methods such as `Socket::set_blocking` to set a file descriptor to be nonblocking:

```rust
let l = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
l.set_nonblocking(true).unwrap();
```

### Polling

As mentioned above, after setting a socket’s file descriptor to be non-blocking, you have to manually poll the file descriptor to check if the I/O operation is completed. Under non-blocking mode, the `TcpListener::Accept` method returns `Ok` if the I/O operation is successful or an error with kind `io::ErrorKind::WouldBlock` is returned.

In the following example, we `loop` until the I/O operation is ready by repeatedly calling `accept`:

```rust
let l = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
l.set_nonblocking(true).unwrap();

loop {
		// the accept call
    let res = l.accept();
    match res {
        Ok((stream, _)) => {
						handle_connection(stream);
						break;
				}
        Err(err) => if err.kind() == io::ErrorKind::WouldBlock {},
    }
}
```

While this works, repeatedly calling `accept` in a loop is not ideal. Each call to `TcpListener::accept` is an expensive call to the kernel.

This is where system calls like [select](http://man7.org/linux/man-pages/man2/select.2.html), [poll,](http://man7.org/linux/man-pages/man2/poll.2.html) [epoll](http://man7.org/linux/man-pages/man7/epoll.7.html), [aio](https://man7.org/linux/man-pages/man7/aio.7.html), [io_uring](https://man.archlinux.org/man/io_uring.7.en) come in. These calls let you register interest for file descriptors and let you know when one or more of them are ready. This reduces the need for constant polling and makes better use of system resources.

Glommio uses `io_uring`. One of the things that make `io_uring` stand out compared to other system calls is that it presents a uniform interface for both sockets and files. This is a huge improvement from system calls like `epoll` that doesn’t support files while `aio` only works with a subset of files (linus-aio only supports `O_DIRECT` files). In the next page, we take a quick glance at how `io_uring` works.
