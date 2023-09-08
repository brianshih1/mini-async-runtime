pub mod source;
mod uring;
pub(crate) use self::{source::*, uring::*};

#[derive(Debug)]
pub(crate) enum SourceType {
    PollableFd,
}
