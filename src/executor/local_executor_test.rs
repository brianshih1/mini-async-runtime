use super::local_executor::LocalExecutor;

#[test]
fn simple_run() {
    let local_ex = LocalExecutor::default();
    let res = local_ex.run(async { 1 + 2 });
    assert_eq!(res, 3);
}
