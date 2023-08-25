use crate::executor::spawn_local;

use super::local_executor::LocalExecutor;

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
        handle.await.unwrap()
    });
    assert_eq!(res, 6)
}
