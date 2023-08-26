use core::fmt;
use std::{
    sync::atomic::{AtomicI16, Ordering},
    task::Waker,
};

use super::{
    raw::TaskVTable,
    state::{CLOSED, COMPLETED},
};

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

    /// The task that is blocked on the `JoinHandle`.
    ///
    /// This waker needs to be woken up once the task completes or is closed.
    pub(crate) awaiter: Option<Waker>,
}

impl Header {
    /// Cancels the task.
    ///
    /// This method will mark the task as closed, but it won't reschedule the
    /// task or drop its future.
    pub(crate) fn cancel(&mut self) {
        // If the task has been completed or closed, it can't be canceled.
        if self.state & (COMPLETED | CLOSED) != 0 {
            return;
        }

        // Mark the task as closed.
        self.state |= CLOSED;
    }

    /// Notifies the awaiter blocked on this task.
    ///
    /// If the awaiter is the same as the current waker, it will not be
    /// notified.
    #[inline]
    pub(crate) fn notify(&mut self, current: Option<&Waker>) {
        // Take the waker out.
        let waker = self.awaiter.take();

        if let Some(w) = waker {
            w.wake()
        }
    }

    /// Registers a new awaiter blocked on this task.
    ///
    /// This method is called when `JoinHandle` is polled and the task has not
    /// completed.
    #[inline]
    pub(crate) fn register(&mut self, waker: &Waker) {
        // Put the waker into the awaiter field.
        self.awaiter = Some(waker.clone());
    }
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
