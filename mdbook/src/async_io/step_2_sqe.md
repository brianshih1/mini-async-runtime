# Step 2 - Submitting a SQE

The second step to asynchronous I/O is to ask `io_uring` to monitor a file descriptor on when it’s ready to perform I/O by submitting a `SQE` entry.

### API

`Async` has two methods which will perform an I/O operation and wait until it is completed:

```rust
// Performs a read I/O operation and wait until it is readable
pub async fn read_with<R>(&self, op: impl FnMut(&T) -> io::Result<R>) 
	-> io::Result<R>

// Performs a write I/O operation and wait until it is writable
pub async fn write_with<R>(&self, op: impl FnMut(&T) -> io::Result<R>) 
	-> io::Result<R>
```

For example, here is how you can use the `read_with` method to implement `Async<TcpListener>`'s `accept` method:

```rust
impl Async<TcpListener> {
    ...

    pub async fn accept(&self) -> io::Result<(Async<TcpStream>, SocketAddr)> {
        let (stream, addr) = self.read_with(|io| io.accept()).await?;
        ...
    }
}
```

### Implementation

Here is the implementation of `read_with`:

```rust
impl<T> Async<T> {
    ...

    pub async fn read_with<R>(&self, op: impl FnMut(&T) -> io::Result<R>) -> io::Result<R> {
        let mut op = op;
        loop {
            match op(self.get_ref()) {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => { }
                res => return res,
            }
						// this waits until the I/O operation is readable (completed)
            self.source.readable().await?; 
        }
    }

		pub fn get_ref(&self) -> &T {
        self.io.as_ref().unwrap()
    }
}
```

It first performs the I/O operation via the call to `op(self.get_ref())`. It then waits for the I/O operation is completed with `self.source.readable().await`.

`Source::readable` is an `async` method that does a few things:

- It stores the `waker` of the `Poller` by invoking `self.add_waiter(cx.waker().clone())`. This way, when the executor detects that the I/O operation is completed, it can invoke `wake()` on the stored waker. The mechanism for waking up the unblocked task is explained in the next page.
- It adds a `SQE` to the `io_uring` instance in the Reactor by calling `get_reactor().sys.interest(self, true, false)`.

Here is the implementation of `Source::readable`:

```rust
impl Source {
    ...

    /// Waits until the I/O source is readable.
    pub(crate) async fn readable(&self) -> io::Result<()> {
        future::poll_fn(|cx| {
            if self.take_result().is_some() {
                return Poll::Ready(Ok(()));
            }

            self.add_waiter(cx.waker().clone());
            get_reactor().sys.interest(self, true, false);
            Poll::Pending
        })
        .await
    }

		pub(crate) fn take_result(&self) -> Option<io::Result<usize>> {
        self.inner.borrow_mut().wakers.result.take()
    }

		pub(crate) fn add_waiter(&self, waker: Waker) {
        self.inner.borrow_mut().wakers.waiters.push(waker);
    }
}
```

Here is the implementation of the `Reactor::interest` method invoked. It first computes the [PollFlags](https://github.com/nix-rust/nix/blob/b28132b7fb7c71e0cc4acc801b5e91e5e769ad47/src/poll.rs#L125) that will be used to construct the `SQE`. It then calls `queue_request_into_ring` to add a `SQE` entry to the submission queue.

```rust
impl Reactor {
    ...

    pub(crate) fn interest(&self, source: &Source, read: bool, write: bool) {
        let mut flags = common_flags();
        if read {
            flags |= read_flags();
        }
        if write {
            flags |= write_flags();
        }

        queue_request_into_ring(
            &mut *self.main_ring.borrow_mut(),
            source,
            UringOpDescriptor::PollAdd(flags),
            &mut self.source_map.clone(),
        );
    }
}

/// Epoll flags for all possible readability events.
fn read_flags() -> PollFlags {
    PollFlags::POLLIN | PollFlags::POLLPRI
}

/// Epoll flags for all possible writability events.
fn write_flags() -> PollFlags {
    PollFlags::POLLOUT
}
```

**queue_request_into_ring**

This method simply adds a `UringDescriptor` onto the `SleepableRing`'s queue. Note that queueing the request into ring doesn’t actually add a `SQE` to the `io_uring`'s submission_queue. It just adds it to the `submission_queue` property on the `SleepableRing`.

```rust
fn queue_request_into_ring(
    ring: &mut (impl UringCommon + ?Sized),
    source: &Source,
    descriptor: UringOpDescriptor,
    source_map: &mut Rc<RefCell<SourceMap>>,
) {
    let q = ring.submission_queue();

    let id = source_map.borrow_mut().add_source(source, Rc::clone(&q));

    let mut queue = q.borrow_mut();

    queue.submissions.push_back(UringDescriptor {
        args: descriptor,
        fd: source.raw(),
        user_data: id,
    });
}
```

Each `UringDescriptor` contains all the information required to fill a `SQE`. For example, since invoking `io_uring_prep_write` requires providing a buffer to write data from, its corresponding `UringOpDescriptor::Write` requires providing a pointer and size for the buffer.

```rust
struct SleepableRing {
    ring: iou::IoUring,
    in_kernel: usize,
    submission_queue: ReactorQueue,
    name: &'static str,
    source_map: Rc<RefCell<SourceMap>>,
}

pub(crate) type ReactorQueue = Rc<RefCell<UringQueueState>>;

pub(crate) struct UringQueueState {
    submissions: VecDeque<UringDescriptor>,
    cancellations: VecDeque<UringDescriptor>,
}

pub(crate) struct UringDescriptor {
    fd: RawFd,
    user_data: u64,
    args: UringOpDescriptor,
}

#[derive(Debug)]
enum UringOpDescriptor {
    PollAdd(PollFlags),
		Write(*const u8, usize, u64),
		...
}
```

Each `UringDescriptor` has a unique `user_data` field. This is the same `user_data` field on each `SQE` and is passed as-is from the `SQE` to the `CQE`. To generate a unique Id, the `add_source` method returns a new unique Id by incrementing a counter each time `add_source` is called:

```rust
impl SourceMap {
	  ...

    fn add_source(&mut self, source: &Source, queue: ReactorQueue) -> u64 {
        let id = self.id;
        self.id += 1;

        self.map.insert(id, source.inner.clone());
        id
    }
```

**Submitting the Events**

Consuming the event is performed by the `consume_submission_queue` method, which calls `consume_sqe_queue`. It repeatedly calls `prep_one_event` to add a `SQE` entry on the `io_uring`'s submission queue by calling `prepare_sqe` to allocate a new `SQE` and calling `fill_sqe` to fill in the necessary details.

If `dispatch` is true, it then calls `submit_sqes` which finally sends the `SQE`s to the kernel.

```rust
impl UringCommon for SleepableRing {
		fn consume_submission_queue(&mut self) -> io::Result<usize> {
        let q = self.submission_queue();
        let mut queue = q.borrow_mut();
        self.consume_sqe_queue(&mut queue.submissions, true)
    }

		fn consume_sqe_queue(
		        &mut self,
		        queue: &mut VecDeque<UringDescriptor>,
		        mut dispatch: bool,
		    ) -> io::Result<usize> {
		        loop {
		            match self.prep_one_event(queue) {
		                None => {
		                    dispatch = true;
		                    break;
		                }
		                Some(true) => {}
		                Some(false) => break,
		            }
		        }
		        if dispatch {
		            self.submit_sqes()
		        } else {
		            Ok(0)
		        }
		    }
			
			fn prep_one_event(&mut self, queue: &mut VecDeque<UringDescriptor>) -> Option<bool> {
	        if queue.is_empty() {
	            return Some(false);
	        }
	
	        if let Some(mut sqe) = self.ring.sq().prepare_sqe() {
	            let op = queue.pop_front().unwrap();
	            // TODO: Allocator
	            fill_sqe(&mut sqe, &op);
	            Some(true)
	        } else {
	            None
	        }
	    }
			
	    fn submit_sqes(&mut self) -> io::Result<usize> {
	        let x = self.ring.submit_sqes()? as usize;
	        self.in_kernel += x;
	        Ok(x)
	    }
}

fn fill_sqe(sqe: &mut iou::SQE<'_>, op: &UringDescriptor) {
    let mut user_data = op.user_data;
    unsafe {
        match op.args {
            UringOpDescriptor::PollAdd(flags) => {
                sqe.prep_poll_add(op.fd, flags);
            }
						...
        }
        sqe.set_user_data(user_data);
    }
}
```
