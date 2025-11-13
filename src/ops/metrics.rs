//! src/ops/metrics.rs
//!
//! Provides basic metrics collection functionality.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A simple, thread-safe metrics collector for operational counters.
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    counters: Arc<Mutex<HashMap<String, u64>>>,
}

impl Metrics {
    /// Creates a new `Metrics` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increments a named counter by one.
    pub fn inc_count(&self, name: &str) {
        let mut counters = self.counters.lock().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Gets the current value of a named counter. Returns 0 if the counter does not exist.
    pub fn get_count(&self, name: &str) -> u64 {
        let counters = self.counters.lock().unwrap();
        *counters.get(name).unwrap_or(&0)
    }
}
