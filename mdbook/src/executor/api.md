# API

Each asynchronous runtime needs an executor to manage tasks. Most asynchronous runtimes implicitly create an executor for you.

For example, in Tokio an executor is created implicitly through `#[tokio::main]`.

```
#[tokio::main]
async fn main() {
    println!("Hello world");
}
```

Under the hood, the annotation actually creates the excutor with something like:

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

In Node.js, the entire application runs on a single event loop. The event loop is initialized when the `node` command is run. 

In Tokio and Node.js, the developer can write asynchronous code without ever knowing the existence of the executor. With `mini-glommio`, developers need to create the executor explicitly.

The two main APIs of our executor are:

- **run**: spawns a task onto the executor and wait until it completes
- **spawn**: spawns a task onto the executor


Pretty simple right? All we need is the ability to put a task onto the executor and to run the task until completion.


### Run

To run a task, you call the `run` method, which is a synchronous method and runs the task until completion.

Here is its signature:

```rust
pub fn run<T>(&self, future: impl Future<Output = T>) -> T 
```

Here is a simple example of using the APIs to run a simple task that performs arithmetics:

```rust
let local_ex = LocalExecutor::default();
let res = local_ex.run(async { 1 + 2 });
assert_eq!(res, 3)
```

### spawn

The whole point of an asynchronous runtime is to perform multitasking. The `spawn` method
allows the programmer to spawn a task onto the executor without waiting for it to complete.

```rust
 pub(crate) fn spawn<T>(&self, future: impl Future<Output = T>) -> JoinHandle<T>
```

The `spawn` method returns a `JoinHandle` which is a future that returns the output of the task
when it completes.

Note that the `spawn` method can technically be run outside a `run` block. However, that means
the programmer would need to manually `poll` the `JoinHandle` to wait until it completes or use another
executor to poll the `JoinHandle`.

Running `spawn` inside the `run` block allows the programmer to just `await` the `JoinHandle`.

Here is an example for how to use `spawn`.

```rust
let local_ex = LocalExecutor::default();
let res = local_ex.run(async {
    let first = local_ex.spawn(async_fetch_value());
		let second = local_ex.spawn(async_fetch_value_2());
    first.await.unwrap() + second.await.unwrap()
});
```

### spawn_local_into

This is a more advanced API that gives a developer more control over the priority of tasks. Instead of placing all the tasks onto a single `TaskQueue` (which is just a collection of tasks), we can instead create different task queues and place each task into one of the queues.

The developer can then set configurations that control how much CPU share each task queue gets.

To create a task queue and spawn a task onto that queue, we can invoke the `spawn_into` method as follows:

```rust
local_ex.run(async {
		let task_queue_handle = executor().create_task_queue(...);
		let task = local_ex.spawn_into(async { write_file().await }, task_queue_handle);
	}
)
```

Next, I will cover the Rust primitives that our executor uses - Future, Async/Await, and Waker. Feel free to skip if you are already familiar with these.
However, if you are not familiar with them, even if you aren't interested in Rust, I strongly advice understanding them as those concepts are
crucial in understanding how asynchronous runtimes work under the hood.