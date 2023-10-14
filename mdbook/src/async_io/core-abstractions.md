# Core abstractions

In general, we can break down how the executor performs asynchronous I/O into 3 steps:

- setting the I/O handle to be non-blocking by setting the `O_NONBLOCK` flag on the file descriptor
- performing the non-blocking operation and registering interest in `io_uring` by submitting a `SQE` to the `io_uring` instance's `submission_queue`
- polling the `io_uring`'s completion queue to check if there is a corresponding `CQE`, which indicates that the I/O operation has been completed. If it's completed, process it by resuming the blocked task.

To accomplish these, we will introduce a few new abstractions: `Async`, `Source`, and the `Reactor`.

### Async

Async is a wrapper around the I/O handle (e.g. TcpListener). It contains helper methods to make converting blocking operations into asynchronous operations easier.

Here is the `Async` struct:

```rust
pub struct Async<T> {
    /// A source registered in the reactor.
    source: Source,

    /// The inner I/O handle.
    io: Option<Box<T>>,
}
```

### Source

The `Source` is a bridge between the executor and the I/O handle. It contains properties pertaining to the I/O handle that are relevant to the executor. For example, it contains tasks that are blocked by operations on the I/O handle.

```rust
pub struct Source {
    pub(crate) inner: Pin<Rc<RefCell<InnerSource>>>,
}

/// A registered source of I/O events.
pub(crate) struct InnerSource {
    /// Raw file descriptor on Unix platforms.
    pub(crate) raw: RawFd,

    /// Tasks interested in events on this source.
    pub(crate) wakers: Wakers,

    pub(crate) source_type: SourceType,
		
		...
}
```

### Reactor

Each executor has a `Reactor`. The `Reactor` is an abstraction around the `io_uring` instance. It provides simple APIs to interact with the `io_uring` instance.

```rust
pub(crate) struct Reactor {
		// the main_ring contains an io_uring instance
    main_ring: RefCell<SleepableRing>,
    source_map: Rc<RefCell<SourceMap>>,
}

struct SleepableRing {
    ring: iou::IoUring,
    in_kernel: usize,
    submission_queue: ReactorQueue,
    name: &'static str,
    source_map: Rc<RefCell<SourceMap>>,
}

struct SourceMap {
    id: u64,
    map: HashMap<u64, Pin<Rc<RefCell<InnerSource>>>>,
}
```

As we can see, the `Reactor` holds a `SleepableRing`, which is just a wrapper around an `iou::IoUring` instance. Glommio uses the `[iou` crate](https://docs.rs/iou/latest/iou/) to interact with Linux kernel’s `io_uring` interface.

The `Reactor` also contains a `SourceMap`, which contains a `HashMap` that maps a unique ID to a `Source`. The unique ID is the same ID used as the `SQE`'s user_data. This way, when a CQE is posted to the `io_uring`'s completion queue, we can tie it back to the corresponding `Source`.
