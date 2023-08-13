use std::{cell::RefCell, collections::BinaryHeap, rc::Rc};

use super::task_queue::TaskQueue;

#[derive(Debug)]
pub(crate) struct QueueManager {
    pub active_executors: BinaryHeap<Rc<RefCell<TaskQueue>>>,
    pub active_executing: Option<Rc<RefCell<TaskQueue>>>,
}

impl QueueManager {
    pub(crate) fn activate_queue(&mut self, queue: Rc<RefCell<TaskQueue>>) {
        todo!()
    }
}
