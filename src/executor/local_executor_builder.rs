use std::path::Iter;

use super::{local_executor::LocalExecutor, placement::Placement};

pub(crate) struct LocalExecutorBuilder {
    placement: Placement,
}

impl LocalExecutorBuilder {
    pub fn new(placement: Placement) -> LocalExecutorBuilder {
        LocalExecutorBuilder { placement }
    }

    pub fn build(self) -> LocalExecutor {
        let cpu_binding = match self.placement {
            Placement::Unbound => None::<Vec<usize>>,
            Placement::Fixed(cpu) => Some(vec![cpu]),
        };
        let mut ex = LocalExecutor::new(cpu_binding);
        ex.init();
        ex
    }
}
