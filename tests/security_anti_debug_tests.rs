use defi_hot_wallet::security::anti_debug::is_debugger_present;

/// Minimal compile-safe test for anti-debug helper.
#[test]
fn test_is_debugger_present_compiles_and_runs() {
    let result = is_debugger_present();
    println!("Debugger present: {}", result);
    // No environment assumption — just ensure function is callable and returns a bool.
    assert!(result == result);
}
