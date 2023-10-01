# Life of a Task

This page aims to explain the execution of a task, following the code paths through the various parts of the executor.

```rust
let local_ex = LocalExecutor::default();
let res = local_ex.run(async {
    let handle = spawn_local({ async_read_file(...).await });
    handle.await
});
```

### Spawning the Task

When the `LocalExecutor` is created, a default `TaskQueue` [is created](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L28). When `local_ex.run(...)` is called, the executor [spawns a task](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L74) with the Future created from the `async` block. It [creates a task](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/task_queue.rs#L116) and [schedules the task](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/task_queue.rs#L117) onto the default TaskQueue. Let’s call this task `Task1`.

### Running Task1

Spawning the task would [create a JoinHandle](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L74C13-L74C71) for `Task1`. The `LocalExecutor` creates a loop that will only exit when `Task1` is completed. The executor verifies when the task is completed by [polling the JoinHandle](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L77C41-L77C52). If it’s completed, the loop exits, and the output of the task is returned. Otherwise, the executor begins [running tasks from active task queues](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L84).

To run the task, the executor would go through all the `TaskQueue`s and execute all the tasks in them. It does so by [creating an outer loop that loops through the`TaskQueue`s](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L99) and [creating an inner loop that runs all the tasks](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L123) in each TaskQueue.

To run a task, the executor [pops the task from the task queue](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L127C40-L127C40) and [runs it](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L127). When the task is [run](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L297), it [creates a Waker](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L323) with the `RAW_WAKER_VTABLE`. Let’s call the created Waker `Waker1`. `Waker1`'s responsibility is to reschedule `Task1` onto the `TaskQueue` when `wake()` is called.

Next, the executor [polls the user-provided Future](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L327) with `Waker1`. As a reminder, the user-provided Future is the Future created from the following `async` block:

```rust
async {
    let handle = spawn_local(async { async_read_file(...).await });
    handle.await
}
```

When the Future is `poll`ed, it would first spawn a task with the Future created from `async { async_read_file(...).await }`. Let’s call the spawned task `Task2`. Spawning `Task2` would also create a `JoinHandle` for it.

Next, `handle.await` is called, which would `poll` the `JoinHandle`. Since `Task2` is not complete, [the waker is registered as Task2’s awaiter](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/join_handle.rs#L52). This `waker` corresponds to `Waker1`. The idea is that `Task2` is blocking `Task1`. So when `Task2` completes, `Waker1::wake()` would be invoked. This would [notify](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/join_handle.rs#L61) the executor that `Task1` is ready to progress again by scheduling `Task1` onto the `TaskQueue`.

### Running Task2

After `Task1::run()` completes, we are back to [the inner loop](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L123) that runs all the tasks from the active TaskQueue. Since `Task2` is now in the `TaskQueue`, the executor would pop it off from the `TaskQueue` to execute it.

When `Task2` is run, a `Waker` for `Task2` is created. Let’s call it `Waker2`. Next, the Future created from `async { async_read_file(...).await }` would be `poll`ed with `Waker2`. Since we haven’t covered how `I/O` works, let’s treat `async_read_file` as a black box. All we need to know is that when the operation is completed, `Waker2::wake()` will be invoked which will reschedule `Task2`.

After `async_read_file` is completed, `Task2` is rescheduled back on the `TaskQueue`. We are back on [the inner loop](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L123) that runs the default `TaskQueue`. It would pop `Task2` off the `TaskQueue` and `poll` it. This time, the `Future` is completed. This would [notify `Task1`](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L364) that `Task2` has been completed by [waking up `Waker1`](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/header.rs#L63). This would reschedule `Task1` and push it back onto the `TaskQueue`.

### Completing Task1

We are back to [the loop](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L123) that runs the default TaskQueue. It would pop `Task1` from the `TaskQueue` and run it. It would `poll` the `Future` which would return `Poll::Ready`. Finally, we can exit both the [inner loop](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L123) and [the outer loop](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L99) since there are no more tasks in any of the `TaskQueue`s to run.

After `run_task_queues` [finishes executing](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L84), the executor would `[poll` `Task1`'s `JoinHandle` again](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/executor/local_executor.rs#L77), which would return `Poll::Pending`. Then the executor can finally return the output result.
