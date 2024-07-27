use std::{io, rc::Rc};

use crate::executor::get_reactor;

/// Waits for a notification.
#[derive(Debug)]

pub(crate) struct Parker {
}

impl Parker {
    /// Creates a new parker.
    pub(crate) fn new() -> Parker {
        // Ensure `Reactor` is initialized now to prevent it from being initialized in
        // `Drop`.
        Parker {
        }
    }

    /// Blocks until notified and then goes back into sleeping state.
    pub(crate) fn park(&self) -> io::Result<bool> {
        todo!()
    }

    /// Performs non-sleepable pool and install a preemption timeout into the
    /// ring with `Duration`. A value of zero means we are not interested in
    /// installing a preemption timer. Tasks executing in the CPU right after
    /// this will be able to check if the timer has elapsed and yield the
    /// CPU if that is the case.
    pub(crate) fn poll_io(&self) -> io::Result<bool> {
        get_reactor().react()
    }
}
