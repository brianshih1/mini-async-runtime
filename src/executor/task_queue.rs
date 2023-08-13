use std::{cell::RefCell, collections::VecDeque, future::Future, rc::Rc};

use crate::task::{
    join_handle::JoinHandle,
    task::{create_task, Task},
};

/// Wrapper around an index that uniquely identifies a TaskQueue
pub struct TaskQueueHandle {
    index: usize,
}

#[derive(Debug)]
pub(crate) struct TaskQueue {
    // contains the actual queue of Tasks
    pub(crate) ex: Rc<TaskQueueExecutor>,
}

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

#[derive(Debug)]
pub(crate) struct TaskQueueExecutor {
    local_queue: LocalQueue,
    name: String,
}

impl TaskQueueExecutor {
    pub(crate) fn new(name: &str) -> Self {
        TaskQueueExecutor {
            local_queue: LocalQueue::new(),
            name: name.into(),
        }
    }

    // Creates a Task with the Future and push it onto the queue by scheduling
    fn create_task<T>(
        &self,
        executor_id: usize,
        tq: Rc<RefCell<TaskQueue>>,
        future: impl Future<Output = T>,
    ) -> (Task, JoinHandle<T>) {
        let tq = Rc::downgrade(&tq);
        let schedule = |task| {
            let tq = tq.upgrade();

            if let Some(tq) = tq {
                {
                    let queue = tq.borrow_mut();
                    let foo = queue.ex;
                    foo.as_ref().local_queue.push(task);
                }
                // TODO: maybe_activate?
            }
        };
        create_task(executor_id, future, schedule)
    }

    pub(crate) fn spawn_and_schedule<T>(
        &self,
        executor_id: usize,
        tq: Rc<RefCell<TaskQueue>>,
        future: impl Future<Output = T>,
    ) -> JoinHandle<T> {
        let (task, handle) = self.create_task(executor_id, tq, future);
        task.schedule();
        handle
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

    pub(crate) fn push(&self, task: Task) {
        self.queue.borrow_mut().push_back(task);
    }

    pub(crate) fn pop(&self) -> Option<Task> {
        self.queue.borrow_mut().pop_front()
    }
}
