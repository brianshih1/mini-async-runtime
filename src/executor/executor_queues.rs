use std::{cell::RefCell, collections::BinaryHeap, rc::Rc};

use super::task_queue::TaskQueue;

struct ExecutorQueues {
    active_executors: BinaryHeap<Rc<RefCell<TaskQueue>>>,
}

impl ExecutorQueues {}
