use std::{future::Future, marker::PhantomData, ptr::NonNull};

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
        todo!()
    }
}

impl<R> Drop for JoinHandle<R> {
    fn drop(&mut self) {
        todo!()
    }
}
