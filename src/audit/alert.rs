// ...existing code...
//! Confirmation helper for audit pipeline (placeholder, balanced braces).
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmationLevel {
    Acknowledged,
    Pending,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct Confirmation {
    level: ConfirmationLevel,
    message: String,
    timestamp: u64,
}

impl Confirmation {
    pub fn new(level: ConfirmationLevel, message: impl Into<String>) -> Self {
        let ts =
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
        Self { level, message: message.into(), timestamp: ts }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn level(&self) -> &ConfirmationLevel {
        &self.level
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub async fn send(&self) -> Result<()> {
        match &self.level {
            ConfirmationLevel::Rejected => {
                warn!(target = "confirmation", ts = self.timestamp, message = %self.message, "confirmation rejected");
            }
            ConfirmationLevel::Pending => {
                info!(target = "confirmation", ts = self.timestamp, message = %self.message, "confirmation pending");
            }
            ConfirmationLevel::Acknowledged => {
                info!(target = "confirmation", ts = self.timestamp, message = %self.message, "confirmation acknowledged");
            }
        }
        Ok(())
    }
}
// ...existing code...
