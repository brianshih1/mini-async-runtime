use std::net::TcpListener;

use crate::{executor::local_executor::LocalExecutor, pollable::Async};

#[test]
fn simple_tcp_accept() {
    let local_ex = LocalExecutor::default();
    let res = local_ex.run(async {
        let listener = Async::<TcpListener>::bind(([127, 0, 0, 1], 8080)).unwrap();
        let (stream, addr) = listener.accept().await.unwrap();
        println!("Accepted client: {}", addr);
    });
}
