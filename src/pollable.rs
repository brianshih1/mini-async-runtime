use std::{io, os::fd::AsRawFd};

use crate::{executor::get_reactor, sys::source::Source};

#[derive(Debug)]
pub struct Async<T> {
    /// A source registered in the reactor.
    source: Source,

    /// The inner I/O handle.
    io: Option<Box<T>>,
}

impl<T: AsRawFd> Async<T> {
    pub fn new(io: T) -> io::Result<Async<T>> {
        Ok(Async {
            source: get_reactor().insert_pollable_io(io.as_raw_fd()),
            io: Some(Box::new(io)),
        })
    }
}
