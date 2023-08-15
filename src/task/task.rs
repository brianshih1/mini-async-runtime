use std::{future::Future, marker::PhantomData, mem, ptr::NonNull};

use super::{header::Header, join_handle::JoinHandle, raw::RawTask, state::SCHEDULED};

#[derive(Debug)]
pub struct Task {
    // Pointer to the raw task (allocated on heap)
    pub raw_task: NonNull<()>,
}

impl Task {
    pub(crate) fn schedule(self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;
        mem::forget(self);

        unsafe {
            ((*header).vtable.schedule)(ptr);
        }
    }

    pub(crate) fn run(self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;

        // vtable.run will call drop_task manually
        mem::forget(self);
        unsafe {
            ((*header).vtable.run)(ptr);
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *mut Header;

        unsafe {
            // Cancel the task.
            (*header).cancel();

            // Drop the future.
            ((*header).vtable.drop_future)(ptr);

            // Mark the task as unscheduled.
            (*header).state &= !SCHEDULED;

            // Drop the task reference.
            ((*header).vtable.drop_task)(ptr);
        }
    }
}

/// Creates a new local task.
///
/// This constructor returns a [`Task`] reference that runs the future and a
/// [`JoinHandle`] that awaits its result.
///
/// When run, the task polls `future`. When woken up, it gets scheduled for
/// running by the `schedule` function.
///
pub(crate) fn create_task<F, R, S>(
    executor_id: usize,
    future: F,
    schedule: S,
) -> (Task, JoinHandle<R>)
where
    F: Future<Output = R>,
    S: Fn(Task),
{
    let raw_task = RawTask::<_, R, S>::allocate(future, schedule, executor_id);

    let task = Task { raw_task };
    let handle = JoinHandle {
        raw_task,
        _marker: PhantomData,
    };
    (task, handle)
}
