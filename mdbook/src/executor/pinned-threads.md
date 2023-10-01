# Pinned Threads

Our goal is to build a `thread-per-core` executor, but so far we’ve been building an executor that runs on the thread that creates it, which would run on whichever CPU the OS decides. Let’s fix that!

On this page, we will build something like this:

```rust
// The LocalExecutor will now only run on Cpu 0
let builder = LocalExecutorBuilder::new(Placement::Fixed(0));
let local_ex = builder.build();
let res = local_ex.run(async {
   ...
});
```

In this code snippet, we’ve introduced two new abstractions:

- **LocalExecutorBuilder**: A factory used to create a `LocalExecutor`
- **Placement**: Specifies a policy that determines the CPUs that the `LocalExecutor` runs on.

We specify to the `LocalExecutorBuilder` that we want to create an executor that only runs on `CPU 0` by passing it `Placement::Fixed(0)`. Then the executor created from `builder.build()` would only run on Cpu 0.

By creating N executors and binding each executor to a specific CPU, the developer can implement a thread-per-core system.

### Implementation

**LocalExecutor**

To limit the CPUs that the `LocalExecutor` can run on, it now takes a list of `CPU`s as its constructor parameters.

```rust
impl LocalExecutor {
    pub fn new(cpu_binding: Option<impl IntoIterator<Item = usize>>) -> Self {
        match cpu_binding {
            Some(cpu_set) => bind_to_cpu_set(cpu_set),
            None => {}
        }
        LocalExecutor { ... }
    }
```

So how can we constrain the `LocalExecutor` to only run on the specified CPUs? We use Linux’s [sched_setaffinity](https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html) method.

As specified in Linux’s manual page, `After a call to **sched_setaffinity**(), the set of CPUs on which the thread will actually run is the intersection of the set specified in the *mask* argument and the set of CPUs actually present on the system.`.

The method `bind_to_cpu_set` that `LocalExecutor::new` calls basically calls the `sched_setaffinity` method:

```rust
pub(crate) fn bind_to_cpu_set(cpus: impl IntoIterator<Item = usize>) {
    let mut cpuset = nix::sched::CpuSet::new();
    for cpu in cpus {
        cpuset.set(cpu).unwrap();
    }
    let pid = nix::unistd::Pid::from_raw(0);
    nix::sched::sched_setaffinity(pid, &cpuset).unwrap();
}
```

The `pid` is set to `0` because the manual page says that `If *pid* is zero, then the calling thread is used.`

**Placement**

Next, we introduce `Placement`s. A `Placement` is a policy that determines what CPUs the `LocalExecutor` will run on. Currently, there are two `Placement`s. We may add more in *Phase 4*.

```rust
pub enum Placement {
    /// The `Unbound` variant creates a [`LocalExecutor`]s that are not bound to
    /// any CPU.
    Unbound,
    /// The [`LocalExecutor`] is bound to the CPU specified by
    /// `Fixed`.
    Fixed(usize),
}
```

`Placement::Unbound` means that the `LocalExecutor` is not bound to any CPU. `Placement::Fixed(cpu_id)` means that the `LoccalExecutor` is bound to the specified CPU.

**LocalExecutorBuilder**

Finally, all the `LocalExecutorBuilder` does is that it transforms a `Placement` into a list of `CPU`s that will be passed into `LocalExecutor`'s constructor.

```rust
pub(crate) struct LocalExecutorBuilder {
    placement: Placement,
}

impl LocalExecutorBuilder {
    pub fn new(placement: Placement) -> LocalExecutorBuilder {
        LocalExecutorBuilder { placement }
    }

    pub fn build(self) -> LocalExecutor {
        let cpu_binding = match self.placement {
            Placement::Unbound => None::<Vec<usize>>,
            Placement::Fixed(cpu) => Some(vec![cpu]),
        };
        let mut ex = LocalExecutor::new(cpu_binding);
        ex.init();
        ex
    }
}
```

When `Placement::Fixed(cpu)` is provided, the `LocalExecutorBuilder` simply creates the `LocalExecutor` with `vec![cpu]` as the specified CPU.
