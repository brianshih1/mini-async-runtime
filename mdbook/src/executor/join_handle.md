# Join Handle

When a task is spawned, the user needs a way to consume the output or cancel the task. This is what the `JoinHandle` does - it allows the user to consume the output of the task or cancel the task.

After a task is spawned, the way to consume the output is to `await` the handle. For example:

```rust
let handle = spawn_local(async { 1 + 3 });
let res: i32 = handle.await;
```

`Await`ing is also a control flow mechanism that allows the user to control the execution order of two tasks. For example, in the following method, the second task won’t be spawned until the first task is completed.

```rust
let handle = spawn_local(...);
handle.await;
spawn_local(...);
```

Since the `JoinHandle` can be `await`ed, it must implement the `Future` trait. So what does the `poll` method of the `JoinHandle` do?

### Poll

`Poll`ing a `JoinHandle` doesn’t actually poll the user-provided future to progress it. The only way for the user-provided future to be `poll`ed is with the `RawTask::run` method which is invoked by the `LocalExecutor`’s `run` method.

Before we look into what `poll` does, let’s first look at the different ways a `JoinHandle` is used.

There are two different ways a `JoinHandle` gets created:

- `LocalExecutor::run`
- `spawn_local` / `spawn_local_into`

**LocalExecutor::run**

Here is a code snippet for the `run` method:

```rust
 LOCAL_EX.set(self, || {
    let waker = dummy_waker();
    let cx = &mut Context::from_waker(&waker);
    let join_handle = self.spawn(async move { future.await });
    pin!(join_handle);
    loop {
        if let Poll::Ready(t) = join_handle.as_mut().poll(cx) {
            return t.unwrap();
        }
        self.run_task_queues();
    }
})
```

We can see that `join_handle` is only used as a way to inspect whether the user-provided future is completed or not. Therefore, a `dummy_waker` is used. A `dummy_waker` is a `Waker` that doesn’t do anything when `wake()` is invoked.

**spawn_local / spawn_local_into**

Earlier, we talked about how the compiler converts the body of an `async` function into a state machine, where each `.await` call represents a new state. We also learned that when the state machine is `poll`ed and it returns `Poll::Pending`, then the executor wouldn’t want to poll the state machine again until the blocking task is completed. Therefore, the blocking task needs to store the waker of the parent task and notify it when the parent task can be `poll`ed again.

This is what the `JoinHandle` created from `spawn_local` and `spawn_local_into` needs to do. It stores the `waker` from the `poll` method and notifies the executor that the parent task can be `poll`ed again.

```rust
let local_ex = LocalExecutor::default();
local_ex.run(async {
    let join_handle = spawn_local(async_write_file());
    join_handle.await;
});
```

In the example above, the `run` method would spawn the `Future` created from the `async` block as follows:

```rust
let join_handle = self.spawn(async move { future.await });
```

Let’s call this `Task A`. When `Task A` gets `poll`ed, it executes the following two lines of code:

```rust
let join_handle = spawn_local(async_write_file());
join_handle.await;
```

Let’s call the task associated with `async_write_file` as `Task B`. When the join handle for `Task B` is `poll`ed, `Task B` is most likely not complete yet. Therefore, `Task B` needs to store the `Waker` from the `poll` method. The `Waker` would schedule `Task A` back onto the executor when `.wake()` is invoked.

### Deep Dive into Poll

Here is the rough structure of the `JoinHandle`'s `poll` method. Notice that the `Output` type is `Option<R>` instead of `R`. The `poll` method returns `Poll::Ready(None)` if the `task` is `CLOSED`. In general, there are three scenarios to cover:

- if the task is `CLOSED`
- if the task is not `COMPLETED`
- if the task is neither `CLOSED` nor not `COMPLETED`

```rust
impl<R> Future for JoinHandle<R> {
    type Output = Option<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *mut Header;

        unsafe {
            let state = (*header).state;

            if state & CLOSED != 0 {
							 ...
            }

            if state & COMPLETED == 0 {
               ...
            }

            ...
        }
    }
}
```

Let’s first look at what happens if the task is `CLOSED`.

```rust
if state & CLOSED != 0 {
    // If the task is scheduled or running, we need to wait until its future is
    // dropped.
    if state & (SCHEDULED | RUNNING) != 0 {
        // Replace the waker with one associated with the current task.
        (*header).register(cx.waker());
        return Poll::Pending;
    }

    // Even though the awaiter is most likely the current task, it could also be
    // another task.
    (*header).notify(Some(cx.waker()));
    return Poll::Ready(None);
}
```

If the task is closed, we notify the awaiter and return `None`. However, in the case that it’s `CLOSED` but still `SCHEDULED | RUNNING`, that means the `future` hasn’t dropped yet. *My understanding of this is that we are playing safe here, but let me know if there’s another reason why we need to return `Poll::Pending` when the future hasn’t dropped yet.*

Next, if the state is not `COMPLETED`, then we simply register the `waker` as the `awaiter` and return `Poll::Pending`.

```rust
if state & COMPLETED == 0 {
    // Replace the waker with one associated with the current task.
    (*header).register(cx.waker());

    return Poll::Pending;
}
```

Finally, in the case that the task’s state is not `CLOSED` and `COMPLETED`, then we mark the task as `CLOSED` since the output has been consumed. We notify the awaiter. And we return `Poll::Ready(Some(output)`.

```rust
(*header).state |= CLOSED;

// Notify the awaiter. Even though the awaiter is most likely the current
// task, it could also be another task.
(*header).notify(Some(cx.waker()));

// Take the output from the task.
let output = ((*header).vtable.get_output)(ptr) as *mut R;
Poll::Ready(Some(output.read()))
```

### Cancel

Another responsibility of `JoinHandle` is that it’s a handle for the user to cancel a task. I won’t go into too much detail about how `cancel` works. But the general idea is that canceling a task means that the future will not be `poll`ed again. However, if the task is already `COMPLETED`, canceling a `JoinHandle` does nothing.

### Code References

To check out my toy implementation or Glommio’s implementation, check out:

**Mini Async Runtime**

- [JoinHandle](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/join_handle.rs#L14)
- [JoinHandle::poll](https://github.com/brianshih1/mini-glommio/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/join_handle.rs#L25)

**Glommio**

- [JoinHandle](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/join_handle.rs#L23)
- [JoinHandle::poll](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/join_handle.rs#L152)
- [JoinHandle::cancel](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/join_handle.rs#L40)
