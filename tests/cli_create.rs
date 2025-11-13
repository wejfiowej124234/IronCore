use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn cli_create_generates_wallet_file() {
    let temp_dir = tempdir().unwrap();
    let unique_name = format!("test-wallet-{}", Uuid::new_v4());
    let file_path = temp_dir.path().join(format!("{}.json", &unique_name));

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "hot_wallet",
            "--features",
            "test-env",
            "--",
            "create",
            "--name",
            &unique_name,
            "--output",
            file_path.to_str().unwrap(),
        ])
        // Run cargo from the repository root so the binary target can be found.
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("DATABASE_URL", "sqlite::memory:")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("created successfully"), "Stdout was: {}", stdout);
    assert!(file_path.exists());

    // Verify file created
    let data = fs::read_to_string(&file_path).expect("read wallet file");
    let v: Value = serde_json::from_str(&data).expect("valid json");
    assert_eq!(v.get("name").and_then(|n| n.as_str()), Some(unique_name.as_str()));
}
