//! Anti-phishing rule engine
//!
//! Rule-based anomaly detection system for identifying common phishing attack patterns

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Threat level classification for transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatLevel {
    /// No threat detected
    None,
    /// Low-level threat
    Low,
    /// Medium-level threat
    Medium,
    /// High-level threat
    High,
    /// Critical threat - should block transaction
    Critical,
}

impl ThreatLevel {
    /// Convert risk score to threat level
    pub fn from_score(score: f64) -> Self {
        if score < 0.2 {
            Self::None
        } else if score < 0.4 {
            Self::Low
        } else if score < 0.6 {
            Self::Medium
        } else if score < 0.8 {
            Self::High
        } else {
            Self::Critical
        }
    }

    /// Check if transaction should be blocked based on threat level
    pub fn should_block(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

/// Anti-phishing rule set configuration
#[derive(Debug, Clone)]
pub struct AntiFishingRules {
    /// Known phishing address blacklist
    blacklist: HashMap<String, String>,
    /// High-value transaction threshold (in native token units)
    high_value_threshold: f64,
    /// Maximum gas price (prevents gas fee fraud)
    max_gas_price: Option<u64>,
    /// List of enabled rules
    #[allow(dead_code)]
    enabled_rules: Vec<RuleType>,
}

/// Types of detection rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleType {
    /// Blacklist address check
    Blacklist,
    /// High-value transfer warning
    HighValueTransfer,
    /// New address interaction warning
    NewAddressInteraction,
    /// Suspicious contract call detection
    SuspiciousContract,
    /// Abnormal gas price detection
    AbnormalGasPrice,
    /// Dust attack detection (frequent small transfers)
    DustAttack,
    /// Unverified token warning
    UnverifiedToken,
}

impl AntiFishingRules {
    /// Create default rule set with standard phishing detection rules
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self {
            blacklist: Self::load_default_blacklist(),
            high_value_threshold: 10.0, // 10 ETH
            max_gas_price: Some(500_000_000_000), // 500 Gwei
            enabled_rules: vec![
                RuleType::Blacklist,
                RuleType::HighValueTransfer,
                RuleType::NewAddressInteraction,
                RuleType::SuspiciousContract,
                RuleType::AbnormalGasPrice,
                RuleType::DustAttack,
                RuleType::UnverifiedToken,
            ],
        }
    }

    /// Load default blacklist (example addresses for demonstration)
    fn load_default_blacklist() -> HashMap<String, String> {
        let mut blacklist = HashMap::new();
        
        // Example phishing address (in production, load from external data sources)
        blacklist.insert(
            "0x0000000000000000000000000000000000000000".to_string(),
            "Null address - common phishing target".to_string(),
        );
        
        // Can integrate with ChainAbuse, ScamSniffer, and other blacklist services
        
        blacklist
    }

    /// Check if address is in the blacklist
    pub fn check_blacklist(&self, address: &str) -> Option<String> {
        self.blacklist.get(address).cloned()
    }

    /// Check if transaction amount exceeds high-value threshold
    pub fn is_high_value_transfer(&self, amount: f64) -> bool {
        amount > self.high_value_threshold
    }

    /// Check if gas price is abnormally high
    pub fn is_abnormal_gas_price(&self, gas_price: u64) -> bool {
        if let Some(max) = self.max_gas_price {
            gas_price > max
        } else {
            false
        }
    }

    /// Add address to blacklist with reason
    pub fn add_to_blacklist(&mut self, address: String, reason: String) {
        self.blacklist.insert(address, reason);
    }

    /// Set high-value transfer threshold
    pub fn set_high_value_threshold(&mut self, threshold: f64) {
        self.high_value_threshold = threshold;
    }
}

/// Rule engine for evaluating transactions against anti-phishing rules
pub struct RuleEngine {
    rules: AntiFishingRules,
    /// Transaction history for pattern analysis
    transaction_history: Vec<TransactionRecord>,
}

#[derive(Debug, Clone)]
pub struct TransactionRecord {
    pub to_address: String,
    pub amount: f64,
    pub timestamp: u64,
    pub is_contract: bool,
}

impl RuleEngine {
    /// Create a new rule engine instance
    pub fn new(rules: AntiFishingRules) -> Self {
        Self {
            rules,
            transaction_history: Vec::new(),
        }
    }

    /// Create a default rule engine with standard configuration
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(AntiFishingRules::default())
    }

    /// Evaluate transaction against all enabled rules
    pub fn evaluate_transaction(
        &mut self,
        to_address: &str,
        amount: f64,
        gas_price: Option<u64>,
        is_contract: bool,
    ) -> (ThreatLevel, Vec<String>) {
        let mut triggered_rules = Vec::new();
        let mut max_threat = ThreatLevel::None;

        // Rule 1: Blacklist check
        if let Some(reason) = self.rules.check_blacklist(to_address) {
            triggered_rules.push(format!("BLACKLIST: {}", reason));
            max_threat = ThreatLevel::Critical;
        }

        // Rule 2: High-value transfer detection
        if self.rules.is_high_value_transfer(amount) {
            triggered_rules.push(format!(
                "HIGH_VALUE: Transfer of {:.4} exceeds threshold",
                amount
            ));
            max_threat = Self::max_threat_level(max_threat, ThreatLevel::Medium);
        }

        // Rule 3: Abnormal gas price detection
        if let Some(gp) = gas_price {
            if self.rules.is_abnormal_gas_price(gp) {
                triggered_rules.push(format!("ABNORMAL_GAS: Gas price {} is too high", gp));
                max_threat = Self::max_threat_level(max_threat, ThreatLevel::High);
            }
        }

        // Rule 4: New address interaction warning
        if !self.has_interacted_before(to_address) && amount > 1.0 {
            triggered_rules.push("NEW_ADDRESS: First interaction with this address".to_string());
            max_threat = Self::max_threat_level(max_threat, ThreatLevel::Low);
        }

        // Rule 5: Suspicious contract detection
        if is_contract && !self.is_verified_contract(to_address) {
            triggered_rules.push("UNVERIFIED_CONTRACT: Interacting with unverified contract".to_string());
            max_threat = Self::max_threat_level(max_threat, ThreatLevel::Medium);
        }

        // Rule 6: Dust attack pattern detection
        if self.is_dust_attack_pattern(to_address, amount) {
            triggered_rules.push("DUST_ATTACK: Suspicious small transfer pattern".to_string());
            max_threat = Self::max_threat_level(max_threat, ThreatLevel::Medium);
        }

        // Record transaction for pattern analysis
        self.record_transaction(TransactionRecord {
            to_address: to_address.to_string(),
            amount,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            is_contract,
        });

        (max_threat, triggered_rules)
    }

    /// Get the higher threat level between two levels
    fn max_threat_level(a: ThreatLevel, b: ThreatLevel) -> ThreatLevel {
        match (a, b) {
            (ThreatLevel::Critical, _) | (_, ThreatLevel::Critical) => ThreatLevel::Critical,
            (ThreatLevel::High, _) | (_, ThreatLevel::High) => ThreatLevel::High,
            (ThreatLevel::Medium, _) | (_, ThreatLevel::Medium) => ThreatLevel::Medium,
            (ThreatLevel::Low, _) | (_, ThreatLevel::Low) => ThreatLevel::Low,
            _ => ThreatLevel::None,
        }
    }

    /// Check if wallet has interacted with this address before
    fn has_interacted_before(&self, address: &str) -> bool {
        self.transaction_history
            .iter()
            .any(|tx| tx.to_address == address)
    }

    /// Check if contract is verified on-chain
    fn is_verified_contract(&self, _address: &str) -> bool {
        // TODO: Integrate on-chain contract verification API
        // Can query Etherscan/BSCScan API for verification status
        false
    }

    /// Detect dust attack patterns (multiple small transfers in short time)
    fn is_dust_attack_pattern(&self, address: &str, amount: f64) -> bool {
        const DUST_THRESHOLD: f64 = 0.001;
        const DUST_COUNT_THRESHOLD: usize = 3;
        const TIME_WINDOW: u64 = 3600; // 1 hour

        if amount > DUST_THRESHOLD {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        // Count recent small transfers to this address within time window
        let recent_dust_count = self
            .transaction_history
            .iter()
            .filter(|tx| {
                tx.to_address == address
                    && tx.amount <= DUST_THRESHOLD
                    && now - tx.timestamp < TIME_WINDOW
            })
            .count();

        recent_dust_count >= DUST_COUNT_THRESHOLD
    }

    /// Record transaction in history for pattern analysis
    fn record_transaction(&mut self, record: TransactionRecord) {
        // Keep only the most recent 1000 transactions to limit memory usage
        const MAX_HISTORY: usize = 1000;
        
        self.transaction_history.push(record);
        
        if self.transaction_history.len() > MAX_HISTORY {
            self.transaction_history.remove(0);
        }
    }

    /// Get immutable reference to rule configuration
    pub fn rules(&self) -> &AntiFishingRules {
        &self.rules
    }

    /// Get mutable reference to rule configuration
    pub fn rules_mut(&mut self) -> &mut AntiFishingRules {
        &mut self.rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_level_from_score() {
        assert_eq!(ThreatLevel::from_score(0.1), ThreatLevel::None);
        assert_eq!(ThreatLevel::from_score(0.3), ThreatLevel::Low);
        assert_eq!(ThreatLevel::from_score(0.5), ThreatLevel::Medium);
        assert_eq!(ThreatLevel::from_score(0.7), ThreatLevel::High);
        assert_eq!(ThreatLevel::from_score(0.9), ThreatLevel::Critical);
    }

    #[test]
    fn test_blacklist_check() {
        let mut engine = RuleEngine::default();
        
        // Test default blacklist detection
        let (threat, rules) = engine.evaluate_transaction(
            "0x0000000000000000000000000000000000000000",
            1.0,
            None,
            false,
        );
        
        assert_eq!(threat, ThreatLevel::Critical);
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_high_value_transfer() {
        let mut engine = RuleEngine::default();
        
        let (threat, rules) = engine.evaluate_transaction(
            "0x1234567890123456789012345678901234567890",
            15.0, // > 10.0 threshold
            None,
            false,
        );
        
        assert!(matches!(threat, ThreatLevel::Medium | ThreatLevel::Low));
        assert!(rules.iter().any(|r| r.contains("HIGH_VALUE")));
    }

    #[test]
    fn test_abnormal_gas_price() {
        let mut engine = RuleEngine::default();
        
        let (threat, rules) = engine.evaluate_transaction(
            "0x1234567890123456789012345678901234567890",
            1.0,
            Some(1_000_000_000_000), // Very high gas
            false,
        );
        
        assert_eq!(threat, ThreatLevel::High);
        assert!(rules.iter().any(|r| r.contains("ABNORMAL_GAS")));
    }

    #[test]
    fn test_new_address_warning() {
        let mut engine = RuleEngine::default();
        
        let (threat, rules) = engine.evaluate_transaction(
            "0xNEWADDRESS",
            2.0,
            None,
            false,
        );
        
        assert!(threat != ThreatLevel::None);
        assert!(rules.iter().any(|r| r.contains("NEW_ADDRESS")));
    }
}

