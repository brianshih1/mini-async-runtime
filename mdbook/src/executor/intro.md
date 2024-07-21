# What is an executor?

As we mentioned earlier, asynchronous programming allows multiple tasks to run on a single thread (or a thread pool). To do that, we need a scheduler
that figures out which task to run at any moment and when to switch tasks. That is what an **executor** does.

There are two main mechanisms in which an executor schedules tasks:

In **preemptive multitasking**, the executor decides when to switch between tasks. It may have an internal timer that forces a task to give up control to the CPU to ensure that each task gets a fair share of the CPU.

In **cooperative multitasking**, the executor lets the task run until it voluntarily gives up control back to the scheduler.

In this section, we will build an executor that performs cooperative multitasking based on `Glommio`'s implementation. Through that,
we will answer the following questions:
- How is a task represented?
- How are the tasks stored in an executor?
- How does a task "give up control" back to the scheduler?

Next, let's look at the API of the executor.