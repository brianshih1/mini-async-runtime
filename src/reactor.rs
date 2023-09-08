use std::{io, os::fd::RawFd};

use crate::{
    executor::executor,
    sys::{self, source::Source, SourceType},
};

/// The reactor.
///
/// Every async I/O handle and every timer is registered here. Invocations of
/// [`run()`][`crate::run()`] poll the reactor to check for new events every now and then.
///
/// There is only one global instance of this type, accessible by [`Local::get_reactor()`].
#[derive(Debug)]

pub(crate) struct Reactor {
    pub(crate) sys: sys::Reactor,
}

impl Reactor {
    pub(crate) fn new(ring_depth: usize) -> Reactor {
        let sys = sys::Reactor::new(ring_depth);
        Self { sys }
    }

    fn new_source(&self, raw: RawFd, stype: SourceType) -> Source {
        // TODO: Task Queue
        Source::new(raw, stype, None)
    }

    pub fn insert_pollable_io(&self, raw: RawFd) -> Source {
        self.new_source(raw, SourceType::PollableFd)
    }

    fn react(&self) -> io::Result<()> {
        self.sys.wait();
        Ok(())
    }
}
