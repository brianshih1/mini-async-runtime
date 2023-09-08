use std::{
    cell::RefCell,
    io,
    os::fd::RawFd,
    pin::Pin,
    rc::Rc,
    task::{Poll, Waker},
};

use futures_lite::future;

use crate::executor::{get_reactor, task_queue::TaskQueueHandle};

use super::SourceType;

#[derive(Debug)]
pub struct Source {
    pub(crate) inner: Pin<Rc<RefCell<InnerSource>>>,
}

impl Source {
    /// Registers an I/O source in the reactor.
    pub(crate) fn new(
        raw: RawFd,
        source_type: SourceType,
        task_queue: Option<TaskQueueHandle>,
    ) -> Source {
        Source {
            inner: Rc::pin(RefCell::new(InnerSource {
                raw,
                wakers: Wakers::new(),
                source_type,
                task_queue,
            })),
        }
    }

    pub(super) fn raw(&self) -> RawFd {
        self.inner.borrow().raw
    }

    /// Waits until the I/O source is readable.
    pub(crate) async fn readable(&self) -> io::Result<()> {
        future::poll_fn(|cx| {
            if self.take_result().is_some() {
                return Poll::Ready(Ok(()));
            }

            self.add_waiter(cx.waker().clone());
            get_reactor().sys.interest(self, true, false);
            Poll::Pending
        })
        .await
    }

    pub(crate) fn take_result(&self) -> Option<io::Result<usize>> {
        self.inner.borrow_mut().wakers.result.take()
    }

    pub(crate) fn add_waiter(&self, waker: Waker) {
        self.inner.borrow_mut().wakers.waiters.push(waker);
    }
}

#[derive(Debug)]
/// A registered source of I/O events.
pub(crate) struct InnerSource {
    /// Raw file descriptor on Unix platforms.
    pub(crate) raw: RawFd,

    /// Tasks interested in events on this source.
    pub(crate) wakers: Wakers,

    pub(crate) source_type: SourceType,

    pub(crate) task_queue: Option<TaskQueueHandle>,
}

/// Tasks interested in events on a source.
#[derive(Debug)]
pub(crate) struct Wakers {
    /// Raw result of the operation.
    pub(crate) result: Option<io::Result<usize>>,

    /// Tasks waiting for the next event.
    pub(super) waiters: Vec<Waker>,
}

impl Wakers {
    pub(super) fn new() -> Self {
        Wakers {
            result: None,
            waiters: Vec::new(),
        }
    }

    pub(super) fn wake_waiters(&mut self) -> bool {
        if self.waiters.is_empty() {
            false
        } else {
            self.waiters.drain(..).for_each(|x| {
                x.wake();
            });
            true
        }
    }
}
