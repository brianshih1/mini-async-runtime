use std::{
    cell::{RefCell, UnsafeCell},
    collections::{HashMap, VecDeque},
    io,
    os::fd::RawFd,
    pin::Pin,
    rc::Rc,
};

use iou::sqe::PollFlags;
use tracing::debug;

use super::source::{InnerSource, Source};

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

impl UringQueueState {
    fn with_capacity(cap: usize) -> ReactorQueue {
        Rc::new(RefCell::new(UringQueueState {
            submissions: VecDeque::with_capacity(cap),
            cancellations: VecDeque::new(),
        }))
    }
}

pub(crate) type ReactorQueue = Rc<RefCell<UringQueueState>>;

pub(crate) trait UringCommon {
    /// None if it wasn't possible to acquire an `sqe`. `Some(true)` if it was
    /// possible and there was something to dispatch. `Some(false)` if there
    /// was nothing to dispatch
    fn prep_one_event(&mut self, queue: &mut VecDeque<UringDescriptor>) -> Option<bool>;
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
            match self.prep_one_event(queue) {
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

    /// Return `None` if no event is completed, `Some(true)` for a task is woken
    /// up and `Some(false)` for not.
    fn consume_one_event(&mut self) -> Option<bool>;

    // TODO: Wakers?
    fn consume_completion_queue(&mut self) -> usize {
        let mut completed: usize = 0;
        loop {
            if self.consume_one_event().is_none() {
                debug!("No events to consume!");
                break;
            } else {
                debug!("We got an event!!");
            }
            completed += 1;
        }
        completed
    }
}

#[derive(Debug)]
struct SleepableRing {
    ring: iou::IoUring,
    in_kernel: usize,
    submission_queue: ReactorQueue,
    name: &'static str,
    source_map: Rc<RefCell<SourceMap>>,
}

impl UringCommon for SleepableRing {
    fn prep_one_event(&mut self, queue: &mut VecDeque<UringDescriptor>) -> Option<bool> {
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

    fn consume_one_event(&mut self) -> Option<bool> {
        let source_map = self.source_map.clone();
        process_one_event(self.ring.peek_for_cqe(), source_map).map(|x| {
            self.in_kernel -= 1;
            x
        })
    }
}

impl SleepableRing {
    fn new(
        size: usize,
        name: &'static str,
        source_map: Rc<RefCell<SourceMap>>,
    ) -> io::Result<Self> {
        Ok(SleepableRing {
            ring: iou::IoUring::new(size as _)?,
            in_kernel: 0,
            submission_queue: UringQueueState::with_capacity(size * 4),
            name,
            source_map,
        })
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

#[derive(Debug)]
pub(crate) struct Reactor {
    main_ring: RefCell<SleepableRing>,
    source_map: Rc<RefCell<SourceMap>>,
}

impl Reactor {
    pub(crate) fn new(ring_depth: usize) -> Reactor {
        let source_map = Rc::new(RefCell::new(SourceMap::new()));
        let main_ring = SleepableRing::new(ring_depth, "main", source_map.clone()).unwrap();
        Reactor {
            main_ring: RefCell::new(main_ring),
            source_map,
        }
    }

    pub(crate) fn interest(&self, source: &Source, read: bool, write: bool) {
        let mut flags = common_flags();
        if read {
            flags |= read_flags();
        }
        if write {
            flags |= write_flags();
        }

        queue_request_into_ring(
            &mut *self.main_ring.borrow_mut(),
            source,
            UringOpDescriptor::PollAdd(flags),
            &mut self.source_map.clone(),
        );
    }

    pub(crate) fn wait(&self) {
        let mut main_ring = self.main_ring.borrow_mut();

        main_ring.consume_completion_queue();
        main_ring.consume_submission_queue().unwrap();
    }
}

fn common_flags() -> PollFlags {
    PollFlags::POLLERR | PollFlags::POLLHUP | PollFlags::POLLNVAL
}

/// Epoll flags for all possible readability events.
fn read_flags() -> PollFlags {
    PollFlags::POLLIN | PollFlags::POLLPRI
}

/// Epoll flags for all possible writability events.
fn write_flags() -> PollFlags {
    PollFlags::POLLOUT
}

fn queue_request_into_ring(
    ring: &mut (impl UringCommon + ?Sized),
    source: &Source,
    descriptor: UringOpDescriptor,
    source_map: &mut Rc<RefCell<SourceMap>>,
) {
    let q = ring.submission_queue();

    let id = source_map.borrow_mut().add_source(source, Rc::clone(&q));

    let mut queue = q.borrow_mut();

    queue.submissions.push_back(UringDescriptor {
        args: descriptor,
        fd: source.raw(),
        user_data: id,
    });
}

#[derive(Debug)]
struct SourceMap {
    id: u64,
    map: HashMap<u64, Pin<Rc<RefCell<InnerSource>>>>,
}

impl SourceMap {
    fn new() -> Self {
        Self {
            id: 1,
            map: HashMap::new(),
        }
    }

    fn add_source(&mut self, source: &Source, queue: ReactorQueue) -> u64 {
        let id = self.id;
        self.id += 1;

        self.map.insert(id, source.inner.clone());
        id
    }

    fn consume_source(&mut self, id: u64) -> Pin<Rc<RefCell<InnerSource>>> {
        let source = self.map.remove(&id).unwrap();
        // let mut s = mut_source(&source);
        // s.id = None;
        // s.queue = None;
        source
    }
}

fn process_one_event(cqe: Option<iou::CQE>, source_map: Rc<RefCell<SourceMap>>) -> Option<bool> {
    if let Some(value) = cqe {
        debug!("There's a CQE!");
        // No user data is `POLL_REMOVE` or `CANCEL`, we won't process.
        if value.user_data() == 0 {
            return Some(false);
        }

        let src = source_map.borrow_mut().consume_source(value.user_data());

        let result = value.result();

        let mut woke = false;

        let mut inner_source = src.borrow_mut();
        inner_source.wakers.result = Some(result.map(|v| v as usize));
        woke = inner_source.wakers.wake_waiters();

        return Some(woke);
    }
    None
}
