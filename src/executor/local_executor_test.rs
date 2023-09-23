use crate::executor::spawn_local;

use super::{
    local_executor::LocalExecutor, local_executor_builder::LocalExecutorBuilder,
    placement::Placement,
};

#[test]
fn simple_run() {
    let local_ex = LocalExecutor::default();
    let res = local_ex.run(async { 1 + 2 });
    assert_eq!(res, 3)
}

#[test]
fn simple_spawn() {
    let local_ex = LocalExecutor::default();
    let res = local_ex.run(async {
        let handle = spawn_local(async { 1 + 5 });
        handle.await.unwrap() + 7
    });
    assert_eq!(res, 13)
}

#[test]
fn local_executor_builder_placement() {
    let builder = LocalExecutorBuilder::new(Placement::Fixed(0));
    let local_ex = builder.build();
    let res = local_ex.run(async {
        let handle = spawn_local(async { 1 + 5 });
        handle.await.unwrap() + 7
    });
    assert_eq!(res, 13)
}
