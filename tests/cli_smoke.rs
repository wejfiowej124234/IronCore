mod util;

use base64::Engine as _;
use std::process::Command;

#[test]
#[ignore = "UTF-8 encoding issues in git history cause gix panic during cargo build"]
fn hot_wallet_refuses_insecure_kek_in_prod() {
    // Ensure we run the binary without the `test-env` feature to simulate production.
    // Supply an all-zero WALLET_ENC_KEY (base64) which the runtime should reject.
    let zeros_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let zeros_b64 = base64::engine::general_purpose::STANDARD.encode(&zeros_vec);

    // Use a deterministic target directory so the compiled binary path is predictable
    // across platforms and avoids hashed names under target/debug/deps.
    let target_dir = std::path::Path::new("target_test_cli_smoke");
    if target_dir.exists() {
        // remove previous build to avoid stale artifacts
        let _ = std::fs::remove_dir_all(target_dir);
    }

    // Build the hot_wallet binary into the deterministic target directory (no test-env feature)
    let build = Command::new("cargo")
        .args(["build", "--bin", "hot_wallet"])
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .expect("failed to run cargo build");
    assert!(
        build.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    // Construct the expected binary path
    let exe_name = if cfg!(windows) { "hot_wallet.exe" } else { "hot_wallet" };
    let bin_path = target_dir.join("debug").join(exe_name);
    assert!(bin_path.exists(), "could not find built hot_wallet binary at {}", bin_path.display());

    let mut run = Command::new(bin_path);
    run.arg("server");
    run.env("WALLET_ENC_KEY", zeros_b64);
    // Ensure the child process truly simulates a production binary by removing
    // any test-only runtime overrides that CI may set (e.g. TEST_SKIP_DECRYPT).
    // If TEST_SKIP_DECRYPT is present in the parent environment (CI/jobs), the
    // binary will bail earlier with a different message and this test will
    // not exercise the insecure-key code path we want to assert on.
    run.env_remove("TEST_SKIP_DECRYPT");

    let output = run.output().expect("failed to spawn hot_wallet binary");
    // The process should exit non-zero (refused to start)
    assert!(!output.status.success(), "Expected hot_wallet to refuse insecure KEK in prod");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);
    assert!(
        combined.contains("Refusing to start: Insecure WALLET_ENC_KEY detected"),
        "binary did not print expected refusal message, combined output:\n{}",
        combined
    );
}
