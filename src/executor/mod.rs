use std::future::Future;

use crate::task::task::Task;

use self::local_executor::LocalExecutor;

pub mod local_executor;

pub mod task_queue;

pub mod executor_queues;

scoped_tls::scoped_thread_local!(static LOCAL_EX: LocalExecutor);

pub(crate) fn executor_id() -> Option<usize> {
    if LOCAL_EX.is_set() {
        Some(LOCAL_EX.with(|ex| ex.get_id()))
    } else {
        None
    }
}

pub(crate) struct ExecutorProxy {}

impl ExecutorProxy {
    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> Task<T>
    where
        T: 'static,
    {
    }
}
