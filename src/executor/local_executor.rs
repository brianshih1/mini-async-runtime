use std::future::Future;

use crate::task::join_handle::JoinHandle;

#[derive(Debug)]
pub struct LocalExecutor {
    id: usize,
}

impl LocalExecutor {
    pub fn get_id(&self) -> usize {
        self.id
    }

    fn spawn<T>(&self, future: impl Future<Output = T>) -> JoinHandle<T> {}

    fn spawn_into<T>(&self, future: impl Future<Output = T>) -> JoinHandle<T> {}
}
