use std::{borrow::BorrowMut, cell::RefCell, collections::VecDeque, future::Future, rc::Rc};

use crate::task::{
    join_handle::JoinHandle,
    task::{create_task, Task},
};

use super::LOCAL_EX;

/// Wrapper around an index that uniquely identifies a TaskQueue
#[derive(Debug)]
pub struct TaskQueueHandle {
    pub(crate) index: usize,
}

#[derive(Debug)]
pub(crate) struct TaskQueue {
    // contains the actual queue of Tasks
    pub(crate) ex: Rc<TaskQueueExecutor>,
    // The invariant around active is that when it's true,
    // it needs to be inside the active_executors
    pub(crate) active: bool,
}

impl Eq for TaskQueue {}

impl Ord for TaskQueue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

impl PartialOrd for TaskQueue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Equal)
    }
}

impl PartialEq for TaskQueue {
    fn eq(&self, other: &Self) -> bool {
        true
    }
}

impl TaskQueue {
    pub(crate) fn new(name: &str) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(TaskQueue {
            ex: Rc::new(TaskQueueExecutor::new(name)),
            active: false,
        }))
    }

    pub fn get_task(&self) -> Option<Task> {
        self.ex.get_task()
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn reset_active(&mut self) {
        self.active = !self.ex.local_queue.is_empty();
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
        let schedule = move |task| {
            let tq = tq.upgrade();

            if let Some(tq) = tq {
                {
                    tq.borrow().ex.as_ref().local_queue.push(task);
                }
                {
                    LOCAL_EX.with(|local_ex| {
                        let mut queues = local_ex.queues.as_ref().borrow_mut();
                        queues.maybe_activate_queue(tq);
                    });
                }
            }
        };
        create_task(executor_id, future, schedule)
    }

    pub fn get_task(&self) -> Option<Task> {
        self.local_queue.pop()
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

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}
