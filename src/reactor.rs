use std::{io, os::fd::RawFd};

use nix::fcntl::{fcntl, FcntlArg, OFlag};

use crate::sys::{self, source::Source, SourceType};

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

    pub fn create_source(&self, raw: RawFd) -> Source {
        fcntl(raw, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();
        self.new_source(raw, SourceType::PollableFd)
    }

    pub fn react(&self) {
        self.sys.wait();
    }
}
