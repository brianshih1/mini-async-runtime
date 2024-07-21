# Thread Pinning

Thread-per-core is a programming paradigm in which developers are not allowed to spawn new threads to run tasks. Instead, each
core only runs a single thread. This is to avoid expensive context switches and avoid using synchronization primitives such as locks.
Check out this excellent [blog](https://www.datadoghq.com/blog/engineering/introducing-glommio/) by Glommio to explain the benefits
of the thread-per-core archicture.

### API

In this section, we will enable the developer to create a `LocalExecutor` that only runs on a particular CPU. In this code snippet below, we create an executor that only runs on `Cpu 0` with the help of the `LocalExecutorBuilder`. 

```rust
// The LocalExecutor will now only run on Cpu 0
let builder = LocalExecutorBuilder::new(Placement::Fixed(0));
let local_ex = builder.build();
let res = local_ex.run(async {
   ...
});
```

By creating N executors and binding each executor to a specific CPU, the developer can implement a thread-per-core system.

### Implementation

**sched_setaffinity**

To force a thread to run on a particular CPU, we will be modifying the thread's CPU affinity mask by using Linux's [sched_affinity](https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html) command. As specified in Linux’s manual page, `After a call to **sched_setaffinity**(), the set of CPUs on which the thread will actually run is the intersection of the set specified in the *mask* argument and the set of CPUs actually present on the system.`.

**LocalExecutor**

We modify `LocalExecutor`'s constructor to take a list of `CPU`s as its parameter. It then calls `bind_to_cpu_set` 

```rust
impl LocalExecutor {
    pub fn new(cpu_binding: Option<impl IntoIterator<Item = usize>>) -> Self {
        match cpu_binding {
            Some(cpu_set) => bind_to_cpu_set(cpu_set),
            None => {}
        }
        LocalExecutor { ... }
    }
  
  	pub(crate) fn bind_to_cpu_set(cpus: impl IntoIterator<Item = usize>) {
        let mut cpuset = nix::sched::CpuSet::new();
        for cpu in cpus {
            cpuset.set(cpu).unwrap();
        }
        let pid = nix::unistd::Pid::from_raw(0);
        nix::sched::sched_setaffinity(pid, &cpuset).unwrap();
    }
  ...
}
```

In `bind_to_cpu_set`, the `pid` is set to `0` because the manual page says that `If *pid* is zero, then the calling thread is used.`

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
