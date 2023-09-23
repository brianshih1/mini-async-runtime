#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Placement {
    /// The `Unbound` variant creates a [`LocalExecutor`]s that are not bound to
    /// any CPU.
    ///
    /// [`LocalExecutor`]: super::LocalExecutor
    /// [`LocalExecutorBuilder`]: super::LocalExecutorBuilder
    Unbound,
    /// The [`LocalExecutor`] is bound to the CPU specified by
    /// `Fixed`.
    ///
    /// #### Errors
    ///
    /// [`LocalExecutorBuilder`] will return `Result::Err` if the CPU doesn't
    /// exist.
    Fixed(usize),
}
