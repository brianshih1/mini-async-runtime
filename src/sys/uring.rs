pub(crate) struct UringDescriptor {
    fd: RawFd,
    flags: SubmissionFlags,
    user_data: u64,
    args: UringOpDescriptor,
}

pub(crate) trait UringCommon {}
