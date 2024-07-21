# Motivation

I've always wondered how asynchronous runtimes like [Node.js](https://nodejs.org/en/about), [Seastar](https://seastar.io/), [Glommio](https://docs.rs/glommio/latest/glommio/), and [Tokio](https://tokio.rs/) work under the hood. Luckily, most async runtimes are open source.
There is also a bunch of excellent blogs online such as the [Asychronous Programming in Rust blog series](https://rust-lang.github.io/async-book/) that I learned from.

In this blog series, I will explain how asynchronous runtimes work under the hood by performing a deep dive into `mini-async-runtime`, a toy asynchronous runtime I built by borrowing snippets from [Glommio](https://github.com/DataDog/glommio) and [async-io](https://github.com/smol-rs/async-io) and stripping them down into a simpler, more managable, async runtime.

Even though this blog series uses a Rust asynchronous runtime as an example, it is meant to be a language-agnostic blog post as most asynchronous runtimes across different languages uses a similar event-loop + reactor architecture. 

## What is an asynchronous runtime?

Synchronous programming is a programming paradigm in which each line of code won't execute until the previous line has completed.
In contrary, asynchronous programming allows the developer to run multiple tasks in parallel through simple primitives such as async/await and futures (or promises in Javascript).

One way that a developer can achieve multitasking without an asynchronous runtime is to use multithreading - just spawn a thread for each task. However, creating a new thread for each task will introduce a bunch of overhead to the system. Each CPU core can only run a task at any given moment. So  the OS will start performing expensive context switches between the threads as the number of threads grow. Also, imagine if you are building a server that can serve millions of request per second. Creating a new thread for each connection will overwhelm the system quickly.

Furthermore, look at how much simpler it is to write concurrent program like the one below as opposed to having to manually create a thread for each task:
```
async function f() {
    const promiseOne = writeToFile()
    const promiseTwo = writeToExternalServer()
    await Promise.all([promise1, promise2, promise3])
}
```

In this example, the two I/O calls are run in parallel. The function will then wait until the two calls complete before exiting.

In other words, an asynchronous runtime is a library that enables multitasking without creating a new thread for each task. It multiplexes multiple
tasks onto a single thread or a thread pool, depending on the implementation.

## What are we building?

I’ve split up the blog series into four phases:

- **Phase 1**: In phase 1, we will build an executor. We will first cover Rust’s asynchronous primitives like `Future`, `Async/Await`, and `Waker` which will serve as building blocks for the asynchronous runtime.
- **Phase 2**: In phase 2, we talk about `io_uring` and use it to add `asynchronous I/O` to our executor
- **Phase 3 [WIP]**: In phase 3, we will implement more advanced features such as thread parking, task yielding, and scheduling tasks based on priority.
- **Phase 4 [WIP]**: In phase 4, we will build more advanced abstractions such as Executor Pools.

As a teaser, here is the architecture of the async runtime that we are building:

<img src="../images/architecture.png" width="110%">