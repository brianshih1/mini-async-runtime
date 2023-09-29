In this blog series, I will explore building a toy version of [Glommio](https://docs.rs/glommio/latest/glommio/), which is an `asynchronous` framework for building `thread-per-core` applications.

### What is Thread-Per-Core?

A complex application may have many tasks that it needs to execute. To speed up the application, some of these tasks can be performed in parallel. The ability of a system to execute multiple tasks concurrently is known as **multitasking**.

Thread-based multitasking is one of the ways an operating system supports multitasking. In thread-based multitasking, an application can spawn a thread for each of its internal tasks. While the CPU can only run one thread at a time, the CPU scheduler can switch between threads to give the user the perception of two or more threads running simultaneously. The switching between threads is known as context switching. 

While thread-based multitasking may allow better usage of the CPU by switching threads when a thread is blocked or waiting, there are a few drawbacks:

- The developer has very little control over which thread is scheduled at any moment. Only a single thread can run on a CPU core at any moment. Once a thread is spawned, it is up to the OS to decide which thread to run on which CPU.
- When the OS switches threads to run on a CPU core, it needs to perform a context switch. A context switch is expensive and may take the kernel around 5 μs to perform.
- If multiple threads try to mutate the same data, they need to use locks to synchronize resource contention. Locks are expensive and threads are blocked while waiting for the lock to be released.

Thread-per-core is an architecture that eliminates threads from the picture. In this programming paradigm, developers are not allowed to spawn new threads to run tasks. Instead, each core runs on a single thread.

[Seastar](https://seastar.io/) (C++) and [Glommio](https://docs.rs/glommio/latest/glommio/) (Rust) are two frameworks that allow developers to write thread-per-core applications. Seastar is used in ScyllaDB and Redpanda while Glommio is used by Datadog.

In this blog series, I will explore building a toy version of Glommio, which is an `asynchronous`, `thread-per-core` runtime based on `io_uring`. I will be building a minimal version of Glommio by extracting bits and pieces from it.

I’ve split up the blog series into four phases:

- **Phase 1**: In phase 1, we will cover Rust’s asynchronous primitives like `Future`, `Async/Await`, and `Waker` which will serve as building blocks for the asynchronous runtime. We will then build a simple, single-threaded, executor that can run and spawn tasks.
- **Phase 2**: In phase 2, we talk about `io_uring` and use it to add `asynchronous I/O` to our executor
- **Phase 3**: In phase 3, we will implement more advanced features such as thread parking, task yielding, and scheduling tasks based on priority.
- **Phase 4**: In phase 4, we will build abstractions that allow developers to create a pool of `LocalExecutor`s.
