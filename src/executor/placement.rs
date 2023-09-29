#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Placement {
    /// The `Unbound` variant creates a [`LocalExecutor`]s that are not bound to
    /// any CPU.
    Unbound,
    /// The [`LocalExecutor`] is bound to the CPU specified by
    /// `Fixed`.
    Fixed(usize),
}
