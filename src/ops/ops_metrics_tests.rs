// filepath: c:\Users\plant\Desktop\Rust鍖哄潡閾綷Defi-Hot-wallet-Rust\tests\ops_metrics_tests.rs

use defi_hot_wallet::ops::metrics::*;
use std::sync::Arc;
use std::thread;

#[test]
fn test_metrics_new_and_get_count() {
    // 姝ｅ父璺緞锛氭祴璇曟柊鍒涘缓鐨?Metrics 瀹炰緥鍜?get_count
    let metrics = Metrics::new();
    assert_eq!(metrics.get_count("non_existent_counter"), 0, "A non-existent counter should return 0");
}

#[test]
fn test_metrics_inc_and_get_count() {
    // 姝ｅ父璺緞锛氭祴璇?inc_count 鍜?get_count
    let metrics = Metrics::new();
    metrics.inc_count("test_counter");
    assert_eq!(metrics.get_count("test_counter"), 1, "Counter should be incremented to 1");

    metrics.inc_count("test_counter");
    assert_eq!(metrics.get_count("test_counter"), 2, "Counter should be incremented to 2");
}

#[test]
fn test_metrics_multiple_counters() {
    // 姝ｅ父璺緞锛氭祴璇曞涓嫭绔嬬殑璁℃暟鍣?
    let metrics = Metrics::new();
    metrics.inc_count("counter_a");
    metrics.inc_count("counter_a");
    metrics.inc_count("counter_b");

    assert_eq!(metrics.get_count("counter_a"), 2);
    assert_eq!(metrics.get_count("counter_b"), 1);
    assert_eq!(metrics.get_count("counter_c"), 0);
}

#[test]
fn test_metrics_thread_safety() {
    // 姝ｅ父璺緞锛氭祴璇曞苟鍙戣闂殑绾跨▼瀹夊叏鎬?
    let metrics = Arc::new(Metrics::new());
    let mut handles = vec![];

    for _ in 0..10 {
        let metrics_clone = Arc::clone(&metrics);
        handles.push(thread::spawn(move || {
            metrics_clone.inc_count("concurrent_counter");
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(metrics.get_count("concurrent_counter"), 10, "Concurrent increments should be correctly handled");
}
