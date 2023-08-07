use std::sync::atomic::AtomicI16;

pub(crate) struct Header {
    pub(crate) state: u8,

    /// Current reference count of the task.
    pub(crate) references: AtomicI16,
}
