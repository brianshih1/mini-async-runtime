# What is an executor?

As we mentioned earlier, the thread-per-core architecture eliminates threads from the picture. Contrary to multithreading applications which let the OS’s CPU scheduler decide which thread to run, thread-per-core frameworks like Glommio built their own **executors** to decide which tasks to run. In other words, an executor is a task scheduler.

An executor needs to decide when to switch between tasks. There are two main ways in which schedulers do that: preemptive multitasking and cooperative multitasking.

In **preemptive multitasking**, the scheduler decides when to switch between tasks. It may have an internal timer that forces a task to give up control to the CPU to ensure that each task gets a fair share of the CPU.

In **cooperative multitasking**, each task runs until it voluntarily gives up control to the scheduler. The type of multitasking Glommio supports is cooperative multitasking.

So how might an executor run tasks? The most simple mechanism is with the event loop.

### The Event Loop

The most simple way in which an executor runs tasks is to use a loop. In each iteration, the executor fetches the tasks and runs all of them sequentially.

```rust
loop {
	let events = executor.get_tasks();
	while !events.is_empty() {
			let task = events.pop();
			executor.process_event(task);
	}
}
```

### Asynchronous Tasks

As you may have noticed, the event loop example above does not support multitasking. It runs each task sequentially until the task is finished or yields control before running the next task. This is where [asynchronous operations](https://en.wikipedia.org/wiki/Asynchronous_I/O) come into play.

When an asynchronous operation is blocked, for example when the operation is waiting for a disk read, it returns a “pending” status to notify the executor that it’s blocked. The executor can then run another task instead of waiting for the blocked operation to complete, wasting its CPU cycles.

In this section, we will build an executor that supports multitasking with the help of asynchronous operations through Rust’s Async / Await primitives.
