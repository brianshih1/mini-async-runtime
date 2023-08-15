use std::future::Future;

use crate::task::{join_handle::JoinHandle, task::Task};

use self::local_executor::LocalExecutor;

pub mod local_executor;
mod local_executor_test;
pub mod queue_manager;
pub mod task_queue;

scoped_tls::scoped_thread_local!(static LOCAL_EX: LocalExecutor);

pub fn spawn_local<T>(future: impl Future<Output = T> + 'static) -> JoinHandle<T>
where
    T: 'static,
{
    executor().spawn_local(future)
}

pub(crate) fn executor_id() -> Option<usize> {
    if LOCAL_EX.is_set() {
        Some(LOCAL_EX.with(|ex| ex.get_id()))
    } else {
        None
    }
}

pub(crate) fn executor() -> ExecutorProxy {
    ExecutorProxy {}
}

pub(crate) struct ExecutorProxy {}

impl ExecutorProxy {
    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> JoinHandle<T>
    where
        T: 'static,
    {
        LOCAL_EX.with(|local_ex| local_ex.spawn(future))
    }
}
