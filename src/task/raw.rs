use std::{
    alloc::{self, Layout},
    future::Future,
    mem,
    ptr::NonNull,
    sync::atomic::{AtomicI16, Ordering},
    task::{RawWaker, RawWakerVTable, Waker},
};

use super::{
    header::Header,
    state::{CLOSED, COMPLETED, HANDLE, RUNNING, SCHEDULED},
    task::Task,
    utils::extend,
};

// VTable for a Task
pub(crate) struct TaskVTable {
    /// Schedules the task.
    pub(crate) schedule: unsafe fn(*const ()),

    /// Drops the future inside the task.
    pub(crate) drop_future: unsafe fn(*const ()),

    /// Returns a pointer to the output stored after completion.
    pub(crate) get_output: unsafe fn(*const ()) -> *const (),

    /// Drops the task.
    pub(crate) drop_task: unsafe fn(ptr: *const ()),

    /// Destroys the task.
    pub(crate) destroy: unsafe fn(*const ()),

    /// Runs the task.
    pub(crate) run: unsafe fn(*const ()) -> bool,
}

/// Raw pointers to the fields inside a task.
pub(crate) struct RawTask<F, R, S> {
    /// The task header.
    pub(crate) header: *const Header,

    /// The schedule function.
    pub(crate) schedule: *const S,

    /// The future.
    pub(crate) future: *mut F,

    /// The output of the future.
    pub(crate) output: *mut R,
}

impl<F, R, S> RawTask<F, R, S>
where
    F: Future<Output = R>,
    S: Fn(Task),
{
    const RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_waker,
        Self::wake,
        Self::wake_by_ref,
        Self::drop_waker,
    );

    pub(crate) fn allocate(future: F, schedule: S, executor_id: usize) -> NonNull<()> {
        let task_layout = Self::task_layout();
        unsafe {
            let raw_task = NonNull::new(alloc::alloc(task_layout.layout) as *mut ()).unwrap();
            let raw = Self::from_ptr(raw_task.as_ptr());
            // Write the header as the first field of the task.
            (raw.header as *mut Header).write(Header {
                state: todo!(),
                executor_id,
                references: AtomicI16::new(0),
                vtable: &TaskVTable {
                    schedule: Self::schedule,
                    drop_future: Self::drop_future,
                    get_output: Self::get_output,
                    drop_task: Self::drop_task,
                    destroy: Self::destroy,
                    run: Self::run,
                },
            });

            // Write the schedule function as the third field of the task.
            (raw.schedule as *mut S).write(schedule);

            // Write the future as the fourth field of the task.
            raw.future.write(future);
            raw_task
        }
    }

    unsafe fn my_executor_id(&self) -> usize {
        (*self.header).executor_id
    }

    pub(crate) fn task_layout() -> TaskLayout {
        // Compute the layouts for `Header`, `T`, `S`, `F`, and `R`.
        let layout_header = Layout::new::<Header>();
        let layout_s = Layout::new::<S>();
        let layout_f = Layout::new::<F>();
        let layout_r = Layout::new::<R>();

        // Compute the layout for `union { F, R }`.
        let size_union = layout_f.size().max(layout_r.size());
        let align_union = layout_f.align().max(layout_r.align());
        let layout_union = unsafe { Layout::from_size_align_unchecked(size_union, align_union) };

        // Compute the layout for `Header` followed by `T`, then `S`, and finally `union
        // { F, R }`.
        let layout = layout_header;
        let (layout, offset_s) = extend(layout, layout_s);
        let (layout, offset_union) = extend(layout, layout_union);
        let offset_f = offset_union;
        let offset_r = offset_union;

        TaskLayout {
            layout,
            offset_s,
            offset_f,
            offset_r,
        }
    }

    /// Creates a `RawTask` from a raw task pointer.
    #[inline]
    pub(crate) fn from_ptr(ptr: *const ()) -> Self {
        let task_layout = Self::task_layout();
        let p = ptr as *const u8;

        unsafe {
            Self {
                header: p as *const Header,
                schedule: p.add(task_layout.offset_s) as *const S,
                future: p.add(task_layout.offset_f) as *mut F,
                output: p.add(task_layout.offset_r) as *mut R,
            }
        }
    }

    unsafe fn destroy(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        let task_layout = Self::task_layout();

        // TODO: We should safeguard against dropping schedule because it
        // contains a closure
        alloc::dealloc(ptr as *mut u8, task_layout.layout);
    }

    fn increment_references(header: &mut Header) {
        let refs = header.references.fetch_add(1, Ordering::Relaxed);
        assert_ne!(refs, i16::MAX, "Waker invariant broken: {:?}", header);
    }

    fn decrement_references(header: &mut Header) -> i16 {
        let refs = header.references.fetch_sub(1, Ordering::Relaxed);
        assert_ne!(refs, 0, "Waker invariant broken: {:?}", header);
        refs - 1
    }

    fn thread_id() -> Option<usize> {
        crate::executor::executor_id()
    }

    /// Wakes a waker. Ptr is the raw task.
    unsafe fn wake_by_ref(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        if Self::thread_id() != Some(raw.my_executor_id()) {
            todo!()
        } else {
            let state = (*raw.header).state;

            // If the task is completed or closed, it can't be woken up.
            if state & (COMPLETED | CLOSED) == 0 {
                // If the task is already scheduled do nothing.
                if state & SCHEDULED == 0 {
                    // Mark the task as scheduled.
                    (*(raw.header as *mut Header)).state = state | SCHEDULED;
                    if state & RUNNING == 0 {
                        // Schedule the task.
                        Self::schedule(ptr);
                    }
                }
            }
        }
    }

    /// Schedules a task for running.
    ///
    /// This function doesn't modify the state of the task. It only passes the
    /// task reference to its schedule function.
    unsafe fn schedule(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        Self::increment_references(&mut *(raw.header as *mut Header));

        // Calling of schedule functions itself does not increment references,
        // if the schedule function has captured variables, increment references
        // so if task being dropped inside schedule function , function itself
        // will keep valid data till the end of execution.
        let guard = if mem::size_of::<S>() > 0 {
            Some(Waker::from_raw(Self::clone_waker(ptr)))
        } else {
            None
        };

        let task = Task {
            raw_task: NonNull::new_unchecked(ptr as *mut ()),
        };

        (*raw.schedule)(task);
        drop(guard);
    }

    /// Clones a waker.
    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let raw = Self::from_ptr(ptr);
        Self::increment_references(&mut *(raw.header as *mut Header));
        RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE)
    }

    /// Wakes a waker. Ptr is the raw task.
    unsafe fn wake(ptr: *const ()) {
        Self::wake_by_ref(ptr);
        Self::drop_waker(ptr);
    }

    /// Drops a waker.
    ///
    /// This function will decrement the reference count. If it drops to
    /// zero, the associated join handle has been dropped too, and the task
    /// has not been completed, then it will get scheduled one more time so
    /// that its future gets dropped by the executor.
    #[inline]
    unsafe fn drop_waker(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        if Self::thread_id() != Some(raw.my_executor_id()) {
            todo!()
        } else {
            let refs = Self::decrement_references(&mut *(raw.header as *mut Header));

            let state = (*raw.header).state;

            // If this was the last reference to the task and the `JoinHandle` has been
            // dropped too, then we need to decide how to destroy the task.
            if (refs == 0) && state & HANDLE == 0 {
                if state & (COMPLETED | CLOSED) == 0 {
                    if state & SCHEDULED == 0 {
                        // If the task was not completed nor closed, close it and schedule one more
                        // time so that its future gets dropped by the
                        // executor.
                        Self::schedule(ptr);
                    }
                    (*(raw.header as *mut Header)).state = SCHEDULED | CLOSED;
                } else {
                    // Otherwise, destroy the task right away.
                    Self::destroy(ptr);
                }
            }
        }
    }

    unsafe fn drop_future(ptr: *const ()) {}

    /// Drops a task.
    ///
    /// This function will decrement the reference count. If it drops to
    /// zero and the associated join handle has been dropped too, then the
    /// task gets destroyed.
    #[inline]
    unsafe fn drop_task(ptr: *const ()) {
        todo!()
    }

    unsafe fn get_output(ptr: *const ()) -> *const () {
        let raw = Self::from_ptr(ptr);
        raw.output as *const ()
    }

    /// Runs a task.
    ///
    /// If polling its future panics, the task will be closed and the panic will
    /// be propagated into the caller.
    unsafe fn run(ptr: *const ()) -> bool {
        todo!()
    }
}

/// Memory layout of a task.
///
/// This struct contains the following information:
///
/// 1. How to allocate and deallocate the task.
/// 2. How to access the fields inside the task.
#[derive(Clone, Copy)]
pub(crate) struct TaskLayout {
    /// Memory layout of the whole task.
    pub(crate) layout: Layout,

    /// Offset into the task at which the schedule function is stored.
    pub(crate) offset_s: usize,

    /// Offset into the task at which the future is stored.
    pub(crate) offset_f: usize,

    /// Offset into the task at which the output is stored.
    pub(crate) offset_r: usize,
}
