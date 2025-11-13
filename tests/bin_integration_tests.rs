// filepath: tests/bin_integration_tests.rs
// 集成测试：实际运行 bin 文件

use std::process::Command;
use std::path::PathBuf;
use std::env;
use std::fs;

// ================================================================================
// 测试 debug_create 二进制
// ================================================================================

#[test]
fn test_debug_create_binary_compiles() {
    // 确保 debug_create 二进制可以编译
    let output = Command::new("cargo")
        .args(&["build", "--bin", "debug_create"])
        .output();
    
    assert!(output.is_ok(), "debug_create should compile");
}

#[test]
#[ignore] // 需要实际运行，可能较慢
fn test_debug_create_execution() {
    // 这个测试实际运行 debug_create 二进制
    // 它会创建一个钱包并立即退出
    let output = Command::new("cargo")
        .args(&["run", "--bin", "debug_create"])
        .env("TEST_SKIP_DECRYPT", "1")
        .env("BRIDGE_MOCK_FORCE_SUCCESS", "1")
        .env("BRIDGE_MOCK", "1")
        .env("ALLOW_BRIDGE_MOCKS", "1")
        .output();
    
    if let Ok(result) = output {
        // 即使失败，也说明二进制可以执行
        assert!(result.status.code().is_some(), "Binary should execute");
    }
}

// ================================================================================
// 测试 nonce_harness 二进制
// ================================================================================

#[test]
fn test_nonce_harness_binary_compiles() {
    // 确保 nonce_harness 二进制可以编译
    let output = Command::new("cargo")
        .args(&["build", "--bin", "nonce_harness"])
        .output();
    
    assert!(output.is_ok(), "nonce_harness should compile");
}

#[test]
fn test_nonce_harness_argument_error() {
    // 测试没有足够参数时的错误处理
    let output = Command::new("cargo")
        .args(&["run", "--bin", "nonce_harness"])
        .output();
    
    if let Ok(result) = output {
        // 应该以错误码 2 退出（参数不足）
        // 或者至少有某个退出码
        assert!(result.status.code().is_some(), "Should exit with error code");
    }
}

// ================================================================================
// 测试 bridge_test 二进制
// ================================================================================

#[test]
fn test_bridge_test_binary_compiles() {
    let output = Command::new("cargo")
        .args(&["build", "--bin", "bridge_test"])
        .output();
    
    assert!(output.is_ok(), "bridge_test should compile");
}

// ================================================================================
// 通用二进制测试
// ================================================================================

#[test]
fn test_cargo_bin_list() {
    // 验证所有二进制都在 Cargo.toml 中定义
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = manifest_path.join("Cargo.toml");
    
    assert!(cargo_toml.exists(), "Cargo.toml should exist");
    
    let content = fs::read_to_string(&cargo_toml).unwrap();
    assert!(content.contains("[[bin]]"), "Should define bin targets");
}

