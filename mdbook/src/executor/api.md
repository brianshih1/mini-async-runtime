# API

An asycnhronous runtime manages tasks. It manages tasks by creating an event loop (or task queue) and an executor which manages the event loop.

Many asynchronous runtimes implicitly create an executor for you.

For example, in Tokio you can create an executor through `#[tokio::main]`.

```
#[tokio::main]
async fn main() {
    println!("Hello world");
}
```

Under the hood, it actually creates an executor via something like:

```
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            println!("Hello world");
        })
}
```

In Node.js, the entire application runs in a single event loop, which is single threaded.

In `mini-glommio`, we will not abstract away executors.

The two main APIs that our executor are:

- **spawn_local**: spawns a task onto the executor
- **run**: runs the task to completion

Pretty simple right? All we need is the ability to put a task onto the executor and wait for the executor to complete.

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

This is a more advanced API that gives a developer more control over the priority of tasks. Instead of placing all the tasks onto a single 
`TaskQueue` (which is just a collection of tasks), we can instead create different task queues and place each task into one of the queues.

The developer can then set configurations that control how much CPU share each task queue gets.

To create a task queue and spawn a task onto that queue, we can invoke the `spawn_local_into` method as follows:

```rust
local_ex.run(async {
		let task_queue_handle = executor().create_task_queue(...);
		let task = spawn_local_into(async { write_file().await }, task_queue_handle);
	}
)
```


Next, I will cover the Rust primitives that our executor uses - Future, Async/Await, and Waker. Feel free to skip if you are already familiar with these.
However, if you are not familiar with them, even if you aren't interested in Rust, I strongly advice understanding them as those concepts are
crucial in understanding how asynchronous runtimes work under the hood.