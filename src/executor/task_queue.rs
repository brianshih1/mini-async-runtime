use std::{cell::RefCell, collections::VecDeque};

use crate::task::task::Task;

/// Wrapper around an index that uniquely identifies a TaskQueue
pub struct TaskQueueHandle {
    index: usize,
}

pub(crate) struct TaskQueue {}

impl PartialOrd for TaskQueue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        todo!()
    }
}

impl PartialEq for TaskQueue {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

pub(crate) struct TaskQueueExecutor {
    local_queue: LocalQueue,
}

impl TaskQueueExecutor {
    pub(crate) fn new() -> Self {
        TaskQueueExecutor {
            local_queue: LocalQueue::new(),
        }
    }
}

#[derive(Debug)]
struct LocalQueue {
    queue: RefCell<VecDeque<Task>>,
}

impl LocalQueue {
    pub(crate) fn new() -> Self {
        LocalQueue {
            queue: RefCell::new(VecDeque::new()),
        }
    }
}
