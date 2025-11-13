use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

struct Metrics {
    inner: Arc<Mutex<HashMap<String, usize>>>,
}

impl Metrics {
    fn new() -> Self {
        Metrics { inner: Arc::new(Mutex::new(HashMap::new())) }
    }
    fn inc_count(&self, key: &str) {
        let mut m = self.inner.lock().unwrap();
        *m.entry(key.to_string()).or_insert(0) += 1;
    }
    fn get_count(&self, key: &str) -> usize {
        let m = self.inner.lock().unwrap();
        *m.get(key).unwrap_or(&0)
    }
}

#[test]
fn test_metrics_new_and_get_count() {
    let metrics = Metrics::new();
    assert_eq!(
        metrics.get_count("non_existent_counter"),
        0,
        "A non-existent counter should return 0"
    );
}

#[test]
fn test_metrics_inc_and_get_count() {
    let metrics = Metrics::new();
    metrics.inc_count("test_counter");
    assert_eq!(metrics.get_count("test_counter"), 1, "Counter should be incremented to 1");
    metrics.inc_count("test_counter");
    assert_eq!(metrics.get_count("test_counter"), 2, "Counter should be incremented to 2");
}

#[test]
fn test_metrics_multiple_counters() {
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
    let metrics = Metrics::new();
    let metrics_arc = Arc::new(metrics);
    let mut handles = vec![];

    for _ in 0..10 {
        let metrics_clone = Arc::clone(&metrics_arc);
        handles.push(thread::spawn(move || {
            metrics_clone.inc_count("concurrent_counter");
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(
        metrics_arc.get_count("concurrent_counter"),
        10,
        "Concurrent increments should be correctly handled"
    );
}
