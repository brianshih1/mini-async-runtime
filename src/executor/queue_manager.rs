use std::{cell::RefCell, collections::BinaryHeap, rc::Rc};

use ahash::AHashMap;

use super::task_queue::TaskQueue;

#[derive(Debug)]
pub(crate) struct QueueManager {
    pub active_queues: BinaryHeap<Rc<RefCell<TaskQueue>>>,
    pub active_executing: Option<Rc<RefCell<TaskQueue>>>,
    pub available_queues: AHashMap<usize, Rc<RefCell<TaskQueue>>>,
}

impl QueueManager {
    pub fn new() -> Self {
        QueueManager {
            active_queues: BinaryHeap::new(),
            active_executing: None,
            available_queues: AHashMap::new(),
        }
    }

    pub(crate) fn maybe_activate_queue(&mut self, queue: Rc<RefCell<TaskQueue>>) {
        let mut state = queue.borrow_mut();
        if !state.is_active() {
            state.active = true;
            drop(state);
            self.active_queues.push(queue);
        }
    }
}
