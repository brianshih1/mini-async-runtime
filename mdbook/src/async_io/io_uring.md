# Io_uring

On this page, I’ll provide a surface-level explanation of how `io_uring` works. If you want a more in-depth explanation, check out [this tutorial](https://unixism.net/loti/what_is_io_uring.html) or [this redhat article](https://developers.redhat.com/articles/2023/04/12/why-you-should-use-iouring-network-io#:~:text=io_uring).

As mentioned, `io_uring` manages file descriptors for the users and lets them know when one or more of them are ready.

Each `io_uring` instance is composed of two ring buffers - the submission queue and the completion queue.

To register interest in a file descriptor, you add an SQE to the tail of the submission queue.  Adding to the submission queue doesn’t automatically send the requests to the kernel, you need to submit it via the `io_uring_enter` system call. `Io_uring` supports batching by allowing you to add multiple SQEs to the ring before submitting.

The kernel processes the submitted entries and adds completion queue events (CQEs) to the completion queue when it is ready. While the order of the CQEs might not match the order of the SQEs, there will be one CQE for each SQE, which you can identify by providing user data.

The user can then check the CQE to see if there are any completed I/O operations.

### Using io_uring for TcpListener

Let’s look at how we can use `IoUring` to manage the `accept` operation for a `TcpListener`. We will be using the `iou` crate, a library built on top of `liburing`, to create and interact with `io_uring` instances.

```rust
let l = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
l.set_nonblocking(true).unwrap();
let mut ring = iou::IoUring::new(2).unwrap();

unsafe {
    let mut sqe = ring.prepare_sqe().expect("failed to get sqe");
    sqe.prep_poll_add(l.as_raw_fd(), iou::sqe::PollFlags::POLLIN);
    sqe.set_user_data(0xDEADBEEF);
    ring.submit_sqes().unwrap();
}
l.accept();
let cqe = ring.wait_for_cqe().unwrap();
assert_eq!(cqe.user_data(), 0xDEADBEEF);
```

In this example, we first create a `[TcpListener](<https://doc.rust-lang.org/stable/std/net/struct.TcpListener.html>)` and set it to non-blocking. Next, we create an `io_uring` instance. We then register interest in the socket’s file descriptor by making a call to `prep_poll_add` (a wrapper around Linux’s [io_uring_prep_poll_add](https://man7.org/linux/man-pages/man3/io_uring_prep_poll_add.3.html) call). This adds a `SQE` entry to the submission queue which will trigger a CQE to be posted [when there is data to be read](https://github.com/nix-rust/nix/blob/e7c877abf73f7f74e358f260683b70ce46db13b0/src/poll.rs#L127).

We then call `accept` to accept any incoming TCP connections. Finally, we call `wait_for_cqe`, which returns the next CQE, blocking the thread until one is ready if necessary. If we wanted to avoid blocking the thread in this example, we could’ve called `peek_for_cqe` which peeks for any completed CQE without blocking.

### Efficiently Checking the CQE

You might be wondering - if we potentially need to call `peek_for_cqe()` repeatedly until it is ready, how is this different from calling `listener.accept()` repeatedly?

The difference is that `accept` is a system call while `peek_for_cqe`, which calls `io_uring_peek_batch_cqe` under the hood, is not a system call. This is due to the unique property of `io_uring` such that the completion ring buffer is shared between the kernel and the user space. This allows you to efficiently check the status of completed I/O operations.
