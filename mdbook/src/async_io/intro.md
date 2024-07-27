# What is Asynchronous I/O?

So far, we have built an executor that can spawn and run tasks. However, we haven't talked about how it can perform I/O, such as 
making a network call or reading from disk.

A simple approach to I/O would be to just wait for the I/O operation to complete. But such an approach, called **synchronous I/O** or **blocking I/O** would block the single-threaded executor from performing any other tasks concurrently.

What we want instead is **asynchronous I/O**. In this approach, performing I/O won’t block the calling thread. Instead, the executor switches to other tasks after making nonblocking I/O call and only resume the task when the kernel notifies the executor that the 
I/O operation is complete.

In this section, we discuss how our executor can perform asynchronous I/O. First, let's look at the primitives that enable to do that.