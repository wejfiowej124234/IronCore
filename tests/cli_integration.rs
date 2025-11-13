mod util;

use assert_cmd::Command;

#[test]
fn test_cli_create_wallet() {
    let mut cmd = Command::cargo_bin("wallet-cli").unwrap();
    // Use centralized test env helper (sets WALLET_ENC_KEY, TEST_SKIP_DECRYPT, ALLOW_BRIDGE_MOCKS)
    util::set_test_env();
    // `create` requires a `--name` argument
    cmd.arg("create").arg("--name").arg("cli-integration-test-wallet").assert().success();
}
