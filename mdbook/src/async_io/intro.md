# What is Asynchronous I/O?

In this phase, we will add I/O to our runtime.

A simple approach to I/O would be to just wait for the I/O operation to complete. But such an approach, called **synchronous I/O** or **blocking I/O** would block the single-threaded executor from performing any other tasks concurrently.

What we want instead is **asynchronous I/O**. In this approach, performing I/O won’t block the calling thread. This allows the executor to run other tasks and return to the original task once the I/O operation completes.

Before we implement asynchronous I/O, we need to first look at two things: how to turn an I/O operation to non-blocking and `io_uring`.
