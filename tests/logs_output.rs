use defi_hot_wallet::mvp::generate_log;

#[test]
fn logs_output_contains_message() {
    let log_output = generate_log("Test log message");
    assert!(log_output.contains("Test"));
    let log_output = generate_log("Another test log");
    assert!(log_output.contains("Another"));
}
