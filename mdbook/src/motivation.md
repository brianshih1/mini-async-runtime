# Motivation

I've always wondered how asynchronous runtimes like [Node.js](https://nodejs.org/en/about), [Seastar](https://seastar.io/), [Glommio](https://docs.rs/glommio/latest/glommio/), and [Tokio](https://tokio.rs/) work under the hood. Luckily, there are a bunch of resources online such as the [Asychronous Programming in Rust blog series](https://rust-lang.github.io/async-book/) and all the open source asynchronous runtimes that I can learn from.


## What is an asynchronous runtime?

Synchronous programming is a programming paradigm in which each line of code won't execute until the previous line has completed.
In contrary, asynchronous programming is a paradigm in which the developer can allow multiple tasks to execute in parallel through 
simple primitives such as async/await and futures (or promises in Javascript).

One way that a developer can achieve multitasking without an asynchronous runtime is to use multi-threading - spawn a thread per task. However, creating a new thread for each task will introduce a bunch of overhead to the system. The OS will start performing expensive context switches as the number of threads grow. Also, imagine if you are building a server that can serve millions of request per second. Creating a new thread for each connection will overwhelm the system quickly.

Furthermore, look at how much simpler it is to write concurrent program like the one below as opposed to manually creating a thread for each task:
```
async function f() {
    const promiseOne = writeToFile()
    const promiseTwo = writeToExternalServer()
    await Promise.all([promise1, promise2, promise3])
}
```

In this example, the two I/O calls are run in parallel. The function will then wait until the two calls complete before exiting.

An asynchronous runtime is a library that takes asynchronous code and figures out how to run and manage the asynchronous tasks efficiently.

## What are we building?

After digging around different asynchronous runtimes, I realized that most of them have a similar architecture.
In this blog series, I will deep dive into how [https://github.com/DataDog/glommio](Glommio) works. However, instead of showing you Glommio code,
I will show you snippets from [mini-glommio](https://github.com/brianshih1/mini-glommio) - a toy asynchronous runtime I created that basically
strips down Glommio into a simpler, more managable, version.

I’ve split up the blog series into four phases:

- **Phase 1**: In phase 1, we will cover Rust’s asynchronous primitives like `Future`, `Async/Await`, and `Waker` which will serve as building blocks for the asynchronous runtime. We will then build a simple, single-threaded, executor that can run and spawn tasks.
- **Phase 2**: In phase 2, we talk about `io_uring` and use it to add `asynchronous I/O` to our executor
- **Phase 3 [WIP]**: In phase 3, we will implement more advanced features such as thread parking, task yielding, and scheduling tasks based on priority.
- **Phase 4 [WIP]**: In phase 4, we will build more advanced abstractions such as Executor Pools.