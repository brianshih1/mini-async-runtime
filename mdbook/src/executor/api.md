# API

As we mentioned earlier, an executor is a task scheduler. Therefore, it needs APIs to submit tasks to the executor as well as consume the output of the tasks.

There are 3 main APIs that our executor supports:

- **run**: runs the task to completion
- **spawn_local**: spawns a task onto the executor
- **spawn_local_into**: spawns a task onto a specific task queue

Here is a simple example of using the APIs to run a simple task that performs arithmetics:

```rust
let local_ex = LocalExecutor::default();
let res = local_ex.run(async { 1 + 2 });
assert_eq!(res, 3)
```

### Run

To run a task, you call the `run` method on the executor, which is a synchronous method and runs the task in the form of a Future (which we will cover next) until completion.

Here is its signature:

```rust
pub fn run<T>(&self, future: impl Future<Output = T>) -> T 
```

### spawn_local

To schedule a `task` onto the `executor`, use the `spawn_local` method:

```rust
let local_ex = LocalExecutor::default();
let res = local_ex.run(async {
    let first = spawn_local(async_fetch_value());
		let second = spawn_local(async_fetch_value_2());
    first.await.unwrap() + second.await.unwrap()
});
```

If `spawn_local` isn’t called from a local executor (i.e. inside a `LocalExecutor::run`), it will panic. Here is its signature:

```rust
pub fn spawn_local<T>(future: impl Future<Output = T> + 'static) -> JoinHandle<T>
where
    T: 'static
```

The return type of `spawn_local` is a `JoinHandle`, which is a `Future` that awaits the result of a task. We will cover abstractions like `JoinHandle` in more depth later.

### spawn_local_into

One of the abstractions that we will cover later is a `TaskQueue`. `TaskQueue` is an abstraction of a collection of tasks. In phase 3, we will introduce more advanced scheduling mechanisms that dictate how much time an executor spends on each `TaskQueue`.

A single executor can have many task queues. To specify which `TaskQueue` to spawn a task to, we can invoke the `spawn_local_into` method as follows:

```rust
local_ex.run(async {
		let task_queue_handle = executor().create_task_queue(...);
		let task = spawn_local_into(async { write_file().await }, task_queue_handle);
	}
)
```

