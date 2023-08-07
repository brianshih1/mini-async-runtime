use std::ptr::NonNull;

pub struct Task {
    // Pointer to the raw task (allocated on heap)
    pub raw_task: NonNull<()>,
}
