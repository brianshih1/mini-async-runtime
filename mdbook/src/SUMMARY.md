# Summary

[Motivation](./motivation.md)

---

# Phase 1 - The Executor
- [What is an executor?](./executor/intro.md)
- [API](./executor/api.md)
- [Prerequisites - Rust Primitives](./executor/primitive-intro.md)
  - [Future](./executor/future.md)
  - [Async/Await](./executor/async-await.md)
  - [Waker](./executor/waker.md)
- [Implementation Details](./executor/implementation-details.md)
  - [Architecture](./executor/architecture.md)
  - [Task](./executor/task.md)
  - [Running the Task](./executor/raw_task.md)
  - [TaskQueue](./executor/task_queue.md)
  - [Waker](./executor/waker_implementation.md)
  - [Local Executor](./executor/local_executor.md)
  - [Join Handle](./executor/join_handle.md)
- [Life of a Task](./executor/life-of-a-task.md)
- [Pinned Threads](./executor/pinned-threads.md)

---

# Phase 2 - Asynchronous I/O
- [What is Asynchronous I/O?](./async_io/intro.md)
- [Prerequisites](./async_io/building_blocks.md)
  - [Non-blocking I/O](./async_io/non_blocking_mode.md)
  - [Io_uring](./async_io/io_uring.md)
- [API](./async_io/api.md)
- [Implementation Details](./async_io/implementation_details.md)
  - [Core abstractions](./async_io/core-abstractions.md)
  - [Step 1 - Setting the O_NONBLOCK Flag](./async_io/step_1_ononblock.md)
  - [Step 2 - Submitting a SQE](./async_io/step_2_sqe.md)
  - [Step 3 - Processing the CQE](./async_io/step_3_cqe.md)







