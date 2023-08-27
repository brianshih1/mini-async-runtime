use std::{
    alloc::{self, Layout},
    future::Future,
    mem::{self, ManuallyDrop},
    pin::Pin,
    ptr::NonNull,
    sync::atomic::{AtomicI16, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
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
                state: SCHEDULED | HANDLE,
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
                awaiter: None,
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
        println!("Wake_by_ref");
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

    unsafe fn drop_future(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        raw.future.drop_in_place();
    }

    /// Drops a task.
    ///
    /// This function will decrement the reference count. If it drops to
    /// zero and the associated join handle has been dropped too, then the
    /// task gets destroyed.
    #[inline]
    unsafe fn drop_task(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        // Decrement the reference count.
        let refs = Self::decrement_references(&mut *(raw.header as *mut Header));

        let state = (*raw.header).state;

        // If this was the last reference to the task and the `JoinHandle` has been
        // dropped too, then destroy the task.
        if refs == 0 && state & HANDLE == 0 {
            Self::destroy(ptr);
        }
    }

    unsafe fn get_output(ptr: *const ()) -> *const () {
        let raw = Self::from_ptr(ptr);
        raw.output as *const ()
    }

    /// Runs a task.
    ///
    /// Returns if the task needs to be scheduled again. If it's closed or completed, then return false.
    /// Otherwise, return true
    unsafe fn run(ptr: *const ()) -> bool {
        let raw = Self::from_ptr(ptr);

        let mut state = (*raw.header).state;

        // If the task has already been closed, drop the task reference and return.
        if state & CLOSED != 0 {
            // Drop the future.
            Self::drop_future(ptr);

            // Mark the task as unscheduled.
            (*(raw.header as *mut Header)).state &= !SCHEDULED;

            // Drop the task reference.
            Self::drop_task(ptr);
            return false;
        }

        // Unset the Scheduled bit and set the Running bit
        state = (state & !SCHEDULED) | RUNNING;
        (*(raw.header as *mut Header)).state = state;

        let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(ptr, &Self::RAW_WAKER_VTABLE)));
        let cx = &mut Context::from_waker(&waker);

        // TODO: Guard
        let poll = <F as Future>::poll(Pin::new_unchecked(&mut *raw.future), cx);

        // state could be updated after the poll
        state = (*raw.header).state;

        // ret is true if the task needs to be scheduled again. This happens
        // if the task is not complete and not closed.
        let mut ret = false;
        match poll {
            Poll::Ready(out) => {
                println!("poll is ready");
                Self::drop_future(ptr);
                raw.output.write(out);

                // A place where the output will be stored in case it needs to be dropped.
                let mut output = None;

                // The task is now completed.
                // If the handle is dropped, we'll need to close it and drop the output.
                // We can drop the output if there is no handle since the handle is the
                // only thing that can retrieve the output from the raw task.
                let new = if state & HANDLE == 0 {
                    (state & !RUNNING & !SCHEDULED) | COMPLETED | CLOSED
                } else {
                    (state & !RUNNING & !SCHEDULED) | COMPLETED
                };

                (*(raw.header as *mut Header)).state = new;

                // If the handle is dropped or if the task was closed while running,
                // now it's time to drop the output.
                if state & HANDLE == 0 || state & CLOSED != 0 {
                    // Read the output.
                    output = Some(raw.output.read());
                }

                // Notify the awaiter that the task has been completed.
                (*(raw.header as *mut Header)).notify(None);

                drop(output);
            }
            Poll::Pending => {
                println!("Task is pending");
                // The task is still not completed.

                // If the task was closed while running, we'll need to unschedule in case it
                // was woken up and then destroy it.
                let new = if state & CLOSED != 0 {
                    state & !RUNNING & !SCHEDULED
                } else {
                    state & !RUNNING
                };

                if state & CLOSED != 0 {
                    Self::drop_future(ptr);
                }

                (*(raw.header as *mut Header)).state = new;

                let is_scheduled = state & SCHEDULED;
                println!("Scheduled: {}", is_scheduled);
                // If the task was closed while running, we need to notify the awaiter.
                // If the task was woken up while running, we need to schedule it.
                // Otherwise, we just drop the task reference.
                if state & CLOSED != 0 {
                    println!("err");
                    // Notify the awaiter that the future has been dropped.
                    (*(raw.header as *mut Header)).notify(None);
                } else if state & SCHEDULED != 0 {
                    // The thread that woke the task up didn't reschedule it because
                    // it was running so now it's our responsibility to do so.
                    Self::schedule(ptr);
                    ret = true;
                }
            }
        }

        // references is incremented each time it's scheduled. After it's run,
        // it needs to be dropped to decrement the reference.
        Self::drop_task(ptr);
        ret
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
