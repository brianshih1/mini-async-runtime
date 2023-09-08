use std::{
    io,
    net::{SocketAddr, TcpListener, TcpStream},
};

use crate::pollable::Async;

impl Async<TcpListener> {
    pub fn bind<A: Into<SocketAddr>>(addr: A) -> io::Result<Async<TcpListener>> {
        let addr = addr.into();
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true).unwrap();
        Ok(Async::new(listener)?)
    }

    pub async fn accept(&self) -> io::Result<(Async<TcpStream>, SocketAddr)> {
        let (stream, addr) = self.read_with(|io| io.accept()).await?;
        Ok((Async::new(stream)?, addr))
    }
}
