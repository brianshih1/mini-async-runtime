use std::{cell::RefCell, collections::VecDeque, io, os::fd::RawFd, rc::Rc};

use iou::sqe::PollFlags;

#[derive(Debug)]
pub(crate) struct UringDescriptor {
    fd: RawFd,
    user_data: u64,
    args: UringOpDescriptor,
}

#[derive(Debug)]
enum UringOpDescriptor {
    PollAdd(PollFlags),
}

#[derive(Debug)]
pub(crate) struct UringQueueState {
    submissions: VecDeque<UringDescriptor>,
    cancellations: VecDeque<UringDescriptor>,
}

pub(crate) type ReactorQueue = Rc<RefCell<UringQueueState>>;

pub(crate) trait UringCommon {
    /// None if it wasn't possible to acquire an `sqe`. `Some(true)` if it was
    /// possible and there was something to dispatch. `Some(false)` if there
    /// was nothing to dispatch
    fn submit_one_event(&mut self, queue: &mut VecDeque<UringDescriptor>) -> Option<bool>;
    fn submit_sqes(&mut self) -> io::Result<usize>;

    fn submission_queue(&mut self) -> ReactorQueue;

    fn consume_submission_queue(&mut self) -> io::Result<usize> {
        let q = self.submission_queue();
        let mut queue = q.borrow_mut();
        self.consume_sqe_queue(&mut queue.submissions, true)
    }

    fn consume_sqe_queue(
        &mut self,
        queue: &mut VecDeque<UringDescriptor>,
        mut dispatch: bool,
    ) -> io::Result<usize> {
        loop {
            match self.submit_one_event(queue) {
                None => {
                    dispatch = true;
                    break;
                }
                Some(true) => {}
                Some(false) => break,
            }
        }
        // TODO: Check if there are actually events
        if dispatch {
            self.submit_sqes()
        } else {
            Ok(0)
        }
    }
}

struct PollRing {
    ring: iou::IoUring,
    in_kernel: usize,
    submission_queue: ReactorQueue,
}

impl UringCommon for PollRing {
    fn submit_one_event(&mut self, queue: &mut VecDeque<UringDescriptor>) -> Option<bool> {
        if queue.is_empty() {
            return Some(false);
        }

        if let Some(mut sqe) = self.ring.sq().prepare_sqe() {
            let op = queue.pop_front().unwrap();
            // TODO: Allocator
            fill_sqe(&mut sqe, &op);
            Some(true)
        } else {
            None
        }
    }

    fn submit_sqes(&mut self) -> io::Result<usize> {
        let x = self.ring.submit_sqes()? as usize;
        self.in_kernel += x;
        Ok(x)
    }

    fn submission_queue(&mut self) -> ReactorQueue {
        self.submission_queue.clone()
    }
}

fn fill_sqe(sqe: &mut iou::SQE<'_>, op: &UringDescriptor) {
    let mut user_data = op.user_data;
    unsafe {
        match op.args {
            UringOpDescriptor::PollAdd(flags) => {
                sqe.prep_poll_add(op.fd, flags);
            }
        }
        sqe.set_user_data(user_data);
    }
}
