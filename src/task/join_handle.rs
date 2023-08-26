use std::{future::Future, marker::PhantomData, ptr::NonNull, sync::atomic::Ordering, task::Poll};

use super::{
    header::Header,
    state::{CLOSED, COMPLETED, HANDLE, RUNNING, SCHEDULED},
};

/// A handle that awaits the result of a task.
///
/// This type is a future that resolves to an `Option<R>` where:
///
/// * `None` indicates the task has panicked or was canceled.
/// * `Some(result)` indicates the task has completed with `result` of type `R`.
pub struct JoinHandle<R> {
    /// A raw task pointer.
    pub(crate) raw_task: NonNull<()>,

    /// A marker capturing generic types `R`.
    pub(crate) _marker: PhantomData<R>,
}

impl<R> Future for JoinHandle<R> {
    type Output = Option<R>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        println!("Polling join handle");
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *mut Header;

        unsafe {
            let state = (*header).state;

            // If the task has been closed, notify the awaiter and return `None`.
            if state & CLOSED != 0 {
                // If the task is scheduled or running, we need to wait until its future is
                // dropped.
                if state & (SCHEDULED | RUNNING) != 0 {
                    // Replace the waker with one associated with the current task.
                    (*header).register(cx.waker());
                    return Poll::Pending;
                }

                (*header).notify(Some(cx.waker()));
                return Poll::Ready(None);
            }

            if state & COMPLETED == 0 {
                // Replace the waker with one associated with the current task.
                (*header).register(cx.waker());
                println!("Join Handle's poll PENDING");
                return Poll::Pending;
            }

            (*header).state |= CLOSED;

            // Notify the awaiter. Even though the awaiter is most likely the current
            // task, it could also be another task.
            (*header).notify(Some(cx.waker()));

            // Take the output from the task.
            let output = ((*header).vtable.get_output)(ptr) as *mut R;
            Poll::Ready(Some(output.read()))
        }
    }
}

impl<R> Drop for JoinHandle<R> {
    fn drop(&mut self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *mut Header;

        // A place where the output will be stored in case it needs to be dropped.
        let mut output = None;

        unsafe {
            // Optimistically assume the `JoinHandle` is being dropped just after creating
            // the task. This is a common case, as often users don't wait on the task
            if (*header).state == SCHEDULED | HANDLE {
                (*header).state = SCHEDULED;
                return;
            }

            let state = (*header).state;
            let refs = (*header).references.load(Ordering::Relaxed);

            // If the task has been completed but not yet closed, that means its output
            // must be dropped.
            if state & COMPLETED != 0 && state & CLOSED == 0 {
                // Mark the task as closed in order to grab its output.
                (*header).state |= CLOSED;
                // Read the output.
                output = Some((((*header).vtable.get_output)(ptr) as *mut R).read());

                (*header).state &= !HANDLE;

                // If this is the last reference to the task, we need to destroy it.
                if refs == 0 {
                    ((*header).vtable.destroy)(ptr)
                }
            } else {
                // If this is the last reference to the task, and it's not closed, then
                // close it and schedule one more time so that its future gets dropped by
                // the executor.
                let new = if (refs == 0) & (state & CLOSED == 0) {
                    SCHEDULED | CLOSED
                } else {
                    state & !HANDLE
                };

                (*header).state = new;
                // If this is the last reference to the task, we need to either
                // schedule dropping its future or destroy it.
                if refs == 0 {
                    if state & CLOSED == 0 {
                        let refs = (*header).references.fetch_add(1, Ordering::Relaxed);
                        assert_ne!(refs, i16::max_value());
                        ((*header).vtable.schedule)(ptr);
                    } else {
                        ((*header).vtable.destroy)(ptr);
                    }
                }
            }
        }

        drop(output);
    }
}
