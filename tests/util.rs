// tests/util.rs
// Shared test helpers for integration/unit tests

use std::process::Command;

/// Sets a deterministic, test-only environment for tests that need
/// WALLET_ENC_KEY, TEST_SKIP_DECRYPT and ALLOW_BRIDGE_MOCKS.
/// Call this early in tests that create wallets with quantum_safe=true
/// or spawn child processes that need deterministic keys.
pub fn set_test_env() {
    // Base64 of 32 zero bytes (used only in tests)
    std::env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    std::env::set_var("TEST_SKIP_DECRYPT", "1");
    std::env::set_var("ALLOW_BRIDGE_MOCKS", "1");
    // Mark that the test helper was invoked so runtime code can detect
    // test-constructor-like behavior when running integration tests.
    std::env::set_var("WALLET_TEST_CONSTRUCTOR", "1");
}

/// Example helper to spawn the CLI with test env applied (returns std::process::Child)
#[allow(dead_code)]
pub fn spawn_cli_with_test_env(args: &[&str]) -> std::process::Child {
    set_test_env();
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--bin").arg("wallet-cli");
    for a in args {
        cmd.arg(a);
    }
    // inherit current env which includes test env vars
    cmd.spawn().expect("failed to spawn wallet-cli")
}
