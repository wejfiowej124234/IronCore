use assert_cmd::Command;

#[test]
fn test_main_runs() {
    let mut cmd = Command::cargo_bin("hot_wallet").unwrap();
    cmd.arg("--help").assert().success(); // 娴嬭瘯甯姪杈撳嚭
}
