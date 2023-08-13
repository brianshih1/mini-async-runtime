use std::{cell::RefCell, collections::BinaryHeap, rc::Rc};

use super::task_queue::TaskQueue;

#[derive(Debug)]
pub(crate) struct ExecutorQueues {
    pub active_executors: BinaryHeap<Rc<RefCell<TaskQueue>>>,
    pub active_executing: Option<Rc<RefCell<TaskQueue>>>,
}

impl ExecutorQueues {}
