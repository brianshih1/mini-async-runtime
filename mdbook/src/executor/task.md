# Task

A `task` is the executor's internal representation for a unit of work submitted by the programmer.

A task is created when a programmer spawns a task with a `future`. For example:

```
let fut = async { 1 + 2 };
local_ex.spawn(fut);
```

When `spawn` is called, the executor takes the `future` and creates a `task`. The task stores these properties in addition to the `future`:
- state
- output
- waker
- references

### **State**

There's a couple of additional `state` that the executor needs to keep
track of:

- **SCHEDULED**: set if the task is scheduled for running
- **RUNNING**: running is set when the future is polled.
- **COMPLETED**: a task is completed when polling the future returns `Poll::Ready`. This means that the output is stored inside the task.
- **CLOSED**: if a task is closed, it’s either canceled or the output has been consumed by a JoinHandle. If a task is `CLOSED`, the task’s `future` will never be `poll`ed again so it can be dropped.
- **HANDLE**: set if the JoinHandle still exists.

For a more thorough explanation of the invariants of the state, check out [this code snippet](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/state.rs).

The state of the task is stored as an `u8`. Each of the states is stored as a bit. For example, `SCHEDULED` is `1 << 0` while `HANDLE` is `1 << 4`. 

### **Output**

The task needs to store the output of a Task for the application to await.

```rust
let handle = spawn_local(async { 1 + 2 });
let res = future.await;
```

In this example, the `Task` needs to store the output (which is 3 in this example) to be consumed by an `await`.

### **Awaiter (Waker)**

When the `task` is blocked (e.g. it's blocked by an I/O operation), we want the executor to switch to another task.
But when should the task be scheduled to be run by the executor again?

This is what the `Waker` is for. The executor creates a `Waker` and passes it to the task each time it polls the task.
The task stores the `waker` and invokes `Waker::wake` when it is unblocked. This will place the task back onto the task queue.

The task stores the `Waker` inside the `awaiter` property:
`pub(crate) awaiter: Option<Waker>`

### **References**

The `Task` needs to be deallocated when there is no more need for it. The `Task` is no longer needed if it’s canceled or when it’s completed and the output is consumed. The `task` has a `references` counter and the task is deallocated once the reference is `0`.


### Implementation

The raw task is allocated on the heap as follows:

```rust
pub struct Task {
    // Pointer to the raw task (allocated on heap)
    pub raw_task: NonNull<()>,
}
```

Here is the implementation of `RawTask`. It uses raw pointers 

```rust
pub(crate) struct RawTask<F, R, S> {
    /// The task header.
    pub(crate) header: *const Header,

    /// The schedule function.
    pub(crate) schedule: *const S,

    /// The future.
    pub(crate) future: *mut F,

    /// The output of the future.
    pub(crate) output: *mut R,
}
```

The `Header` contains the `state`, the `references,` and the `awaiter`.

```rust
pub(crate) struct Header {
    pub(crate) state: u8,

    pub(crate) executor_id: usize,

    /// Current reference count of the task.
    pub(crate) references: AtomicI16,

    /// The virtual table.
    pub(crate) vtable: &'static TaskVTable,

    /// The task that is blocked on the `JoinHandle`.
    ///
    /// This waker needs to be woken up once the task completes or is closed.
    pub(crate) awaiter: Option<Waker>,
}
```

Both the `Glommio crate` and the `async_task` crate use the virtual table to contain pointers to methods necessary for bookkeeping the task. My understanding is that this reduces the runtime overhead, but let me know if there are other reasons why!

### Creating a Task

Finally, to create a `Task`, you invoke the `create_task` method:

```rust
pub(crate) fn create_task<F, R, S>(
    executor_id: usize,
    future: F,
    schedule: S,
) -> (Task, JoinHandle<R>)
where
    F: Future<Output = R>,
    S: Fn(Task),
{
    let raw_task = RawTask::<_, R, S>::allocate(future, schedule, executor_id);

    let task = Task { raw_task };
    let handle = JoinHandle {
        raw_task,
        _marker: PhantomData,
    };
    (task, handle)
}
```

The `create_task` method takes a `schedule` function. Usually, the `schedule` method simply places the task onto
the task queue.

```rust
let schedule = move |task| {
    let task_queue = tq.upgrade();
    task_queue.local_queue.push(task);
};
create_task(executor_id, future, schedule)
```

The core of this function is the `allocate` method which allocates the `Task` onto the heap:

```rust
pub(crate) fn allocate(future: F, schedule: S, executor_id: usize) -> NonNull<()> {
  let task_layout = Self::task_layout();
  unsafe {
      let raw_task = NonNull::new(alloc::alloc(task_layout.layout) as *mut ()).unwrap();
      let raw = Self::from_ptr(raw_task.as_ptr());
      // Write the header as the first field of the task.
      (raw.header as *mut Header).write(Header {
          state: SCHEDULED | HANDLE,
          executor_id,
          references: AtomicI16::new(0),
          vtable: &TaskVTable {
              schedule: Self::schedule,
              drop_future: Self::drop_future,
              get_output: Self::get_output,
              drop_task: Self::drop_task,
              destroy: Self::destroy,
              run: Self::run,
          },
          awaiter: None,
      });

      // Write the schedule function as the third field of the task.
      (raw.schedule as *mut S).write(schedule);

      // Write the future as the fourth field of the task.
      raw.future.write(future);
      raw_task
  }
}
```

Note that the initial `state` of a `Task` is `SCHEDULED | HANDLE`. It’s `SCHEDULED` because a task is considered to be scheduled whenever its `Task` reference exists. There’s a `HANDLE` because the `JoinHandle` hasn’t dropped yet.

### API

The two most important APIs of a `Task` are `schedule` and `run`.

**pub(crate) fn schedule(self)**

This method schedules the task. It increments the `references` and calls the `schedule` method stored in the `Task`. In the context of an executor, the `schedule` method pushes itself onto the `Task Queue` that it was originally spawned into.

**pub(crate) fn run(self)**

The `run` method is how the user-provided future gets `poll`ed. Since the `run` method is quite meaty, I will dedicate the entire next page to talk about how it works.

### Code References

To check out my toy implementation or Glommio’s implementation, check out:

**My Toy Implementation**

- [Raw Task](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/raw.rs#L39)
- [State](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/state.rs)
- [Task](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/task.rs#L6)
- [Task::schedule](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/task.rs#L12)
- [Task::run](https://github.com/brianshih1/mini-async-runtime/blob/7025a02d91f19e258d69e966f8dfc98eeeed4ecc/src/task/task.rs#L22)

**Glommio**

- [Raw Task](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/raw.rs#L72)
- [State](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/state.rs)
- [Task](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/task_impl.rs#L53)
- [Task::schedule](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/task_impl.rs#L82)
- [Task::run](https://github.com/DataDog/glommio/blob/d93c460c3def6b11a224892657a6a6a80edf6311/glommio/src/task/task_impl.rs#L98)
