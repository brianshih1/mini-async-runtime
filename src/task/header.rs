use core::fmt;
use std::sync::atomic::{AtomicI16, Ordering};

use super::raw::TaskVTable;

pub(crate) struct Header {
    pub(crate) state: u8,

    pub(crate) executor_id: usize,

    /// Current reference count of the task.
    pub(crate) references: AtomicI16,

    /// The virtual table.
    ///
    /// In addition to the actual waker virtual table, it also contains pointers
    /// to several other methods necessary for bookkeeping the
    /// heap-allocated task.
    ///
    /// The static lifetime guarantees that the reference would be valid
    /// while the Header exists
    pub(crate) vtable: &'static TaskVTable,
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state;
        let refcount = self.references.load(Ordering::Relaxed);
        f.debug_struct("Header")
            .field("ptr", &(self as *const Self))
            .field("refcount", &refcount)
            .finish()
    }
}
