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

impl<T> Async<T> {
    pub async fn readable(&self) -> io::Result<()> {
        self.source.readable().await
    }

    pub fn get_ref(&self) -> &T {
        self.io.as_ref().unwrap()
    }

    pub async fn read_with<R>(&self, op: impl FnMut(&T) -> io::Result<R>) -> io::Result<R> {
        let mut op = op;
        loop {
            match op(self.get_ref()) {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                res => return res,
            }
            self.readable().await?;
        }
    }
}
