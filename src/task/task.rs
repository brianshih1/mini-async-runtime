use std::{future::Future, ptr::NonNull};

use super::join_handle::JoinHandle;

pub struct Task {
    // Pointer to the raw task (allocated on heap)
    pub raw_task: NonNull<()>,
}

/// Creates a new local task.
///
/// This constructor returns a [`Task`] reference that runs the future and a
/// [`JoinHandle`] that awaits its result.
///
/// When run, the task polls `future`. When woken up, it gets scheduled for
/// running by the `schedule` function.
///
pub(crate) fn spawn_local<F, R, S>(
    executor_id: usize,
    future: F,
    schedule: S,
) -> (Task, JoinHandle<R>)
where
    F: Future<Output = R>,
    S: Fn(Task),
{
    todo!()
}
