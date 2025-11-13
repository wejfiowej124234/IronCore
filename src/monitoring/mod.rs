use anyhow::Result;
use prometheus::{Counter, Encoder, Gauge, Histogram, HistogramOpts, Registry, TextEncoder};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use crate::security::redaction::redact_body;

pub struct WalletMetrics {
    registry: Registry,

    // Wallet metrics
    pub wallets_created: Counter,
    pub wallets_accessed: Counter,
    pub wallets_deleted: Counter,

    // Transaction metrics
    pub transactions_sent: Counter,
    pub transactions_failed: Counter,
    pub transaction_value: Histogram,
    pub transaction_fees: Histogram,

    // Security metrics
    pub login_attempts: Counter,
    pub failed_logins: Counter,
    pub quantum_encryptions: Counter,
    pub multisig_operations: Counter,

    // Performance metrics
    pub active_connections: Gauge,
    pub response_time: Histogram,
    pub database_operations: Histogram,

    // Network metrics
    pub blockchain_calls: Counter,
    pub blockchain_errors: Counter,
    pub network_latency: Histogram,
}

impl WalletMetrics {
    pub fn new() -> Result<Self> {
        info!("馃搳 Initializing wallet metrics");

        let registry = Registry::new();

        // Wallet metrics
        let wallets_created =
            Counter::new("wallets_created_total", "Total number of wallets created")?;
        let wallets_accessed =
            Counter::new("wallets_accessed_total", "Total number of wallet accesses")?;
        let wallets_deleted =
            Counter::new("wallets_deleted_total", "Total number of wallets deleted")?;

        // Transaction metrics
        let transactions_sent =
            Counter::new("transactions_sent_total", "Total number of transactions sent")?;
        let transactions_failed =
            Counter::new("transactions_failed_total", "Total number of failed transactions")?;
        let transaction_value = Histogram::with_opts(HistogramOpts::new(
            "transaction_value",
            "Transaction values in native tokens",
        ))?;
        let transaction_fees = Histogram::with_opts(HistogramOpts::new(
            "transaction_fees",
            "Transaction fees in native tokens",
        ))?;

        // Security metrics
        let login_attempts =
            Counter::new("login_attempts_total", "Total number of login attempts")?;
        let failed_logins = Counter::new("failed_logins_total", "Total number of failed logins")?;
        let quantum_encryptions = Counter::new(
            "quantum_encryptions_total",
            "Total number of quantum encryptions performed",
        )?;
        let multisig_operations =
            Counter::new("multisig_operations_total", "Total number of multisig operations")?;

        // Performance metrics
        let active_connections = Gauge::new("active_connections", "Number of active connections")?;
        let response_time = Histogram::with_opts(HistogramOpts::new(
            "response_time_seconds",
            "Response time in seconds",
        ))?;
        let database_operations = Histogram::with_opts(HistogramOpts::new(
            "database_operations_seconds",
            "Database operation time in seconds",
        ))?;

        // Network metrics
        let blockchain_calls =
            Counter::new("blockchain_calls_total", "Total number of blockchain API calls")?;
        let blockchain_errors =
            Counter::new("blockchain_errors_total", "Total number of blockchain API errors")?;
        let network_latency = Histogram::with_opts(HistogramOpts::new(
            "network_latency_seconds",
            "Network latency in seconds",
        ))?;

        // Register all metrics
        registry.register(Box::new(wallets_created.clone()))?;
        registry.register(Box::new(wallets_accessed.clone()))?;
        registry.register(Box::new(wallets_deleted.clone()))?;
        registry.register(Box::new(transactions_sent.clone()))?;
        registry.register(Box::new(transactions_failed.clone()))?;
        registry.register(Box::new(transaction_value.clone()))?;
        registry.register(Box::new(transaction_fees.clone()))?;
        registry.register(Box::new(login_attempts.clone()))?;
        registry.register(Box::new(failed_logins.clone()))?;
        registry.register(Box::new(quantum_encryptions.clone()))?;
        registry.register(Box::new(multisig_operations.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(response_time.clone()))?;
        registry.register(Box::new(database_operations.clone()))?;
        registry.register(Box::new(blockchain_calls.clone()))?;
        registry.register(Box::new(blockchain_errors.clone()))?;
        registry.register(Box::new(network_latency.clone()))?;

        info!("鉁?Wallet metrics initialized");

        Ok(Self {
            registry,
            wallets_created,
            wallets_accessed,
            wallets_deleted,
            transactions_sent,
            transactions_failed,
            transaction_value,
            transaction_fees,
            login_attempts,
            failed_logins,
            quantum_encryptions,
            multisig_operations,
            active_connections,
            response_time,
            database_operations,
            blockchain_calls,
            blockchain_errors,
            network_latency,
        })
    }

    pub fn export_metrics(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    pub fn record_wallet_created(&self) {
        self.wallets_created.inc();
        info!("馃搳 Recorded wallet creation");
    }

    pub fn record_wallet_accessed(&self) {
        self.wallets_accessed.inc();
    }

    pub fn record_wallet_deleted(&self) {
        self.wallets_deleted.inc();
        warn!("馃搳 Recorded wallet deletion");
    }

    pub fn record_transaction_sent(&self, value: f64, fee: f64) {
        self.transactions_sent.inc();
        self.transaction_value.observe(value);
        self.transaction_fees.observe(fee);
        info!("馃搳 Recorded successful transaction: value={}, fee={}", value, fee);
    }

    pub fn record_transaction_failed(&self) {
        self.transactions_failed.inc();
        error!("馃搳 Recorded failed transaction");
    }

    pub fn record_login_attempt(&self, success: bool) {
        self.login_attempts.inc();
        if !success {
            self.failed_logins.inc();
            warn!("馃搳 Recorded failed login attempt");
        }
    }

    pub fn record_quantum_encryption(&self) {
        self.quantum_encryptions.inc();
    }

    pub fn record_multisig_operation(&self) {
        self.multisig_operations.inc();
    }

    pub fn set_active_connections(&self, count: f64) {
        self.active_connections.set(count);
    }

    pub fn record_response_time(&self, duration: f64) {
        self.response_time.observe(duration);
    }

    pub fn record_database_operation(&self, duration: f64) {
        self.database_operations.observe(duration);
    }

    pub fn record_blockchain_call(&self, success: bool, latency: f64) {
        self.blockchain_calls.inc();
        self.network_latency.observe(latency);

        if !success {
            self.blockchain_errors.inc();
            warn!("馃搳 Recorded blockchain API error");
        }
    }
}

pub struct SecurityMonitor {
    #[allow(dead_code)]
    metrics: Arc<WalletMetrics>,
    suspicious_activity: Arc<Mutex<Vec<SecurityEvent>>>,
}

#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub event_type: SecurityEventType,
    pub description: String,
    pub severity: SecuritySeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source_ip: Option<String>,
    pub wallet_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SecurityEventType {
    UnauthorizedAccess,
    SuspiciousTransaction,
    MultipleFailedLogins,
    UnusualLocation,
    QuantumAttackAttempt,
    MalformedRequest,
}

#[derive(Debug, Clone)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl SecurityMonitor {
    pub fn new(metrics: Arc<WalletMetrics>) -> Self {
        info!("馃洝锔?Initializing security monitor");

        Self { metrics, suspicious_activity: Arc::new(Mutex::new(Vec::new())) }
    }

    pub async fn report_security_event(&self, event: SecurityEvent) {
        let severity_str = match event.severity {
            SecuritySeverity::Low => "LOW",
            SecuritySeverity::Medium => "MEDIUM",
            SecuritySeverity::High => "HIGH",
            SecuritySeverity::Critical => "CRITICAL",
        };

        // Map event type to a stable string and redact potentially-sensitive description
        let event_type_str = match event.event_type {
            SecurityEventType::UnauthorizedAccess => "UnauthorizedAccess",
            SecurityEventType::SuspiciousTransaction => "SuspiciousTransaction",
            SecurityEventType::MultipleFailedLogins => "MultipleFailedLogins",
            SecurityEventType::UnusualLocation => "UnusualLocation",
            SecurityEventType::QuantumAttackAttempt => "QuantumAttackAttempt",
            SecurityEventType::MalformedRequest => "MalformedRequest",
        };

        let redacted_description = redact_body(&event.description);
        warn!(
            "馃毃 Security Event [{}]: {} - {}",
            severity_str,
            event_type_str,
            redacted_description
        );

        // Store the event
        let mut events = self.suspicious_activity.lock().await;
        events.push(event.clone());

        // Keep only recent events (last 1000)
        if events.len() > 1000 {
            events.drain(0..100);
        }

        // For critical events, you might want to send alerts
        if matches!(event.severity, SecuritySeverity::Critical) {
            self.send_critical_alert(&event).await;
        }
    }

    pub async fn get_recent_security_events(&self, limit: usize) -> Vec<SecurityEvent> {
        let events = self.suspicious_activity.lock().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    pub async fn check_suspicious_activity(&self, ip: &str, wallet_id: Option<&str>) -> bool {
        let events = self.suspicious_activity.lock().await;
        let recent_threshold = chrono::Utc::now() - chrono::Duration::minutes(15);

        // Check for multiple failed logins from same IP
        let failed_logins = events
            .iter()
            .filter(|e| {
                matches!(e.event_type, SecurityEventType::MultipleFailedLogins)
                    && e.timestamp > recent_threshold
                    && e.source_ip.as_ref() == Some(&ip.to_string())
            })
            .count();

        if failed_logins >= 5 {
            warn!(
                "馃毃 Detected suspicious activity: {} failed logins from IP {}",
                failed_logins, ip
            );
            return true;
        }

        // Check for suspicious transactions if wallet_id is provided
        if let Some(wallet_id) = wallet_id {
            let suspicious_txs = events
                .iter()
                .filter(|e| {
                    matches!(e.event_type, SecurityEventType::SuspiciousTransaction)
                        && e.timestamp > recent_threshold
                        && e.wallet_id.as_ref() == Some(&wallet_id.to_string())
                })
                .count();

            if suspicious_txs >= 3 {
                warn!("馃毃 Detected suspicious transaction activity for wallet {}", wallet_id);
                return true;
            }
        }

        false
    }

    async fn send_critical_alert(&self, event: &SecurityEvent) {
        // For critical events, redact details before logging externally.
        let event_type_str = match event.event_type {
            SecurityEventType::UnauthorizedAccess => "UnauthorizedAccess",
            SecurityEventType::SuspiciousTransaction => "SuspiciousTransaction",
            SecurityEventType::MultipleFailedLogins => "MultipleFailedLogins",
            SecurityEventType::UnusualLocation => "UnusualLocation",
            SecurityEventType::QuantumAttackAttempt => "QuantumAttackAttempt",
            SecurityEventType::MalformedRequest => "MalformedRequest",
        };

        let redacted_desc = redact_body(&event.description);
        error!("馃毃 CRITICAL SECURITY ALERT: {} - {}", event_type_str, redacted_desc);
        // In a real implementation, this would:
        // - Send webhook notifications
        // - Email administrators
        // - Integrate with incident management systems
        // - Possibly auto-lock affected wallets

        // For now, we'll just log it
        // Avoid printing raw wallet IDs or IPs; redact them for logs unless DEV_PRINT_SECRETS=1
        let redacted_ip = event.source_ip.as_deref().map(redact_body);
        let redacted_wallet = event.wallet_id.as_deref().map(redact_body);
        error!(
            "Alert details: IP={:?}, Wallet={:?}, Time={}",
            redacted_ip, redacted_wallet, event.timestamp
        );
    }
}

static METRICS: once_cell::sync::OnceCell<Arc<WalletMetrics>> = once_cell::sync::OnceCell::new();
static SECURITY_MONITOR: once_cell::sync::OnceCell<Arc<SecurityMonitor>> =
    once_cell::sync::OnceCell::new();

pub async fn init_metrics() -> Result<()> {
    let metrics = Arc::new(WalletMetrics::new()?);
    let security_monitor = Arc::new(SecurityMonitor::new(metrics.clone()));

    METRICS.set(metrics).map_err(|_| anyhow::anyhow!("Metrics already initialized"))?;
    SECURITY_MONITOR
        .set(security_monitor)
        .map_err(|_| anyhow::anyhow!("Security monitor already initialized"))?;

    info!("鉁?Monitoring system initialized");
    Ok(())
}

pub fn get_metrics() -> Option<Arc<WalletMetrics>> {
    METRICS.get().cloned()
}

pub fn get_security_monitor() -> Option<Arc<SecurityMonitor>> {
    SECURITY_MONITOR.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = WalletMetrics::new().unwrap();

        // Test recording some metrics
        metrics.record_wallet_created();
        metrics.record_transaction_sent(1.5, 0.001);
        metrics.record_login_attempt(true);
        metrics.record_login_attempt(false);

        // Test exporting metrics
        let exported = metrics.export_metrics().unwrap();
        assert!(exported.contains("wallets_created_total"));
        assert!(exported.contains("transactions_sent_total"));
        assert!(exported.contains("login_attempts_total"));
    }

    #[tokio::test]
    async fn test_security_monitor() {
        let metrics = Arc::new(WalletMetrics::new().unwrap());
        let monitor = SecurityMonitor::new(metrics);

        let event = SecurityEvent {
            event_type: SecurityEventType::UnauthorizedAccess,
            description: "Test unauthorized access".to_string(),
            severity: SecuritySeverity::High,
            timestamp: chrono::Utc::now(),
            source_ip: Some("192.168.1.1".to_string()),
            wallet_id: Some("test-wallet".to_string()),
        };

        monitor.report_security_event(event).await;

        let events = monitor.get_recent_security_events(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].description, "Test unauthorized access");
    }
}
