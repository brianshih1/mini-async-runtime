use std::{cell::RefCell, future::Future, rc::Rc};

use crate::task::join_handle::JoinHandle;

use super::{
    executor_queues::ExecutorQueues,
    task_queue::{TaskQueue, TaskQueueHandle},
};

#[derive(Debug)]
pub struct LocalExecutor {
    id: usize,
    queues: Rc<RefCell<ExecutorQueues>>,
}

impl LocalExecutor {
    pub fn add_default_task_queue() {}

    pub fn get_id(&self) -> usize {
        self.id
    }

    fn get_default_queue(&self) -> Option<Rc<RefCell<TaskQueue>>> {
        self.get_queue(TaskQueueHandle { index: 0 })
    }

    pub(crate) fn get_queue(&self, handle: TaskQueueHandle) -> Option<Rc<RefCell<TaskQueue>>> {
        todo!()
    }

    pub(crate) fn spawn<T>(&self, future: impl Future<Output = T>) -> JoinHandle<T> {
        let tq = self
            .queues
            .borrow()
            .active_executing
            .clone() // this clone is cheap because we clone an `Option<Rc<_>>`
            .or_else(|| self.get_default_queue())
            .unwrap();
        let tq_executor = tq.borrow().ex.clone();
        tq_executor.spawn_and_schedule(self.id, tq, future)
    }

    pub(crate) fn spawn_into<T>(
        &self,
        future: impl Future<Output = T>,
        handle: TaskQueueHandle,
    ) -> JoinHandle<T> {
        todo!()
    }
}
