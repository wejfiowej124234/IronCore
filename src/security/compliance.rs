// src/security/compliance.rs
//! Simple compliance checks (AML / limits) used by wallet operations.

use crate::core::errors::WalletError;
use std::collections::HashMap;

/// Compliance result
#[derive(Debug, Clone, PartialEq)]
pub enum ComplianceResult {
    Compliant,
    NonCompliant(String),
    RequiresApproval(String),
}

/// Transaction types
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionType {
    Transfer,
    Receive,
    Swap,
    Stake,
    Unstake,
    Bridge,
}

/// Risk levels
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Compliance checker
pub struct ComplianceChecker {
    max_daily_limit: f64,
    max_transaction_limit: f64,
    restricted_countries: Vec<String>,
    sanctioned_addresses: Vec<String>,
    user_daily_totals: HashMap<String, f64>,
}

impl ComplianceChecker {
    /// Create a new compliance checker with sensible defaults.
    pub fn new() -> Self {
        Self {
            max_daily_limit: 10_000.0,
            max_transaction_limit: 1_000.0,
            restricted_countries: vec![
                "IR".to_string(), // Iran
                "KP".to_string(), // North Korea
                "CU".to_string(), // Cuba
                "SY".to_string(), // Syria
            ],
            sanctioned_addresses: vec![],
            user_daily_totals: HashMap::new(),
        }
    }

    /// Check a transaction for compliance.
    pub fn check_transaction(
        &mut self,
        user_id: &str,
        transaction_type: &TransactionType,
        amount: f64,
        recipient_address: &str,
        user_country: &str,
    ) -> Result<ComplianceResult, WalletError> {
        // Restricted country check (case-insensitive)
        if self.restricted_countries.iter().any(|c| c.eq_ignore_ascii_case(user_country)) {
            return Ok(ComplianceResult::NonCompliant(format!(
                "Transactions from {} are restricted",
                user_country
            )));
        }

        // Sanctioned recipient check (case-insensitive)
        if self.sanctioned_addresses.iter().any(|a| a.eq_ignore_ascii_case(recipient_address)) {
            return Ok(ComplianceResult::NonCompliant(
                "Recipient address is sanctioned".to_string(),
            ));
        }

        // Single transaction limit check
        if amount > self.max_transaction_limit {
            return Ok(ComplianceResult::RequiresApproval(format!(
                "Transaction amount {} exceeds limit {}",
                amount, self.max_transaction_limit
            )));
        }

        // Daily limit check
        let current_daily = *self.user_daily_totals.get(user_id).unwrap_or(&0.0);
        if current_daily + amount > self.max_daily_limit {
            return Ok(ComplianceResult::RequiresApproval(format!(
                "Daily limit would be exceeded: current {}, adding {}, limit {}",
                current_daily, amount, self.max_daily_limit
            )));
        }

        // Transaction-type specific checks
        match transaction_type {
            TransactionType::Bridge => {
                if amount > self.max_transaction_limit * 0.5 {
                    return Ok(ComplianceResult::RequiresApproval(
                        "Large bridge transactions require approval".to_string(),
                    ));
                }
            }
            TransactionType::Swap => {
                // Placeholder for swap-specific checks
            }
            _ => {}
        }

        // Update daily total and return compliant
        let new_total = current_daily + amount;
        self.user_daily_totals.insert(user_id.to_string(), new_total);

        Ok(ComplianceResult::Compliant)
    }

    /// Assess risk level for a transaction
    pub fn assess_risk(
        &self,
        transaction_type: &TransactionType,
        amount: f64,
        recipient_address: &str,
        user_history: usize,
    ) -> RiskLevel {
        let mut risk_score: i32 = 0;

        // Amount-based scoring
        if amount > self.max_transaction_limit * 5.0 {
            risk_score += 5;
        } else if amount > self.max_transaction_limit {
            risk_score += 3;
        }

        // Transaction type scoring
        match transaction_type {
            TransactionType::Bridge => risk_score += 2,
            TransactionType::Swap => risk_score += 1,
            TransactionType::Stake | TransactionType::Unstake => { /* lower risk */ }
            _ => {}
        }

        // New user scoring
        if user_history < 5 {
            risk_score += 2;
        }

        // Short recipient address heuristic
        if recipient_address.len() < 20 {
            risk_score += 3;
        }

        match risk_score {
            0..=2 => RiskLevel::Low,
            3..=5 => RiskLevel::Medium,
            6..=8 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    /// Reset per-user daily totals (e.g. run nightly)
    pub fn reset_daily_limits(&mut self) {
        self.user_daily_totals.clear();
    }

    /// Add sanctioned address (case-insensitive dedupe)
    pub fn add_sanctioned_address(&mut self, address: String) {
        if !self.sanctioned_addresses.iter().any(|a| a.eq_ignore_ascii_case(&address)) {
            self.sanctioned_addresses.push(address);
        }
    }

    /// Remove sanctioned address
    pub fn remove_sanctioned_address(&mut self, address: &str) {
        self.sanctioned_addresses.retain(|a| !a.eq_ignore_ascii_case(address));
    }

    /// Get user's daily usage
    pub fn get_user_daily_usage(&self, user_id: &str) -> f64 {
        *self.user_daily_totals.get(user_id).unwrap_or(&0.0)
    }

    /// Is address sanctioned (case-insensitive)
    pub fn is_address_sanctioned(&self, address: &str) -> bool {
        self.sanctioned_addresses.iter().any(|a| a.eq_ignore_ascii_case(address))
    }
}

impl Default for ComplianceChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_check() {
        let mut checker = ComplianceChecker::new();
        let user_id = "user123";
        let recipient = "0x1234567890abcdef";
        let country = "US";

        let result = checker
            .check_transaction(user_id, &TransactionType::Transfer, 100.0, recipient, country)
            .unwrap();
        assert_eq!(result, ComplianceResult::Compliant);
        assert_eq!(checker.get_user_daily_usage(user_id), 100.0);
    }

    #[test]
    fn test_transaction_limit() {
        let mut checker = ComplianceChecker::new();
        let user_id = "user123";

        let result = checker
            .check_transaction(
                user_id,
                &TransactionType::Transfer,
                2_000.0,
                "0x1234567890abcdef0000000000000000000000",
                "US",
            )
            .unwrap();

        match result {
            ComplianceResult::RequiresApproval(_) => {}
            _ => panic!("Expected approval required"),
        }
    }

    #[test]
    fn test_restricted_country() {
        let mut checker = ComplianceChecker::new();
        let result = checker
            .check_transaction(
                "user123",
                &TransactionType::Transfer,
                100.0,
                "0x1234567890abcdef0000000000000000000000",
                "IR",
            )
            .unwrap();

        match result {
            ComplianceResult::NonCompliant(_) => {}
            _ => panic!("Expected non-compliant"),
        }
    }

    #[test]
    fn test_risk_assessment() {
        let checker = ComplianceChecker::new();

        let risk = checker.assess_risk(
            &TransactionType::Transfer,
            100.0,
            "0x1234567890abcdef1234567890abcdef",
            10,
        );
        assert_eq!(risk, RiskLevel::Low);

        let risk = checker.assess_risk(&TransactionType::Bridge, 6_000.0, "short", 1);
        assert_eq!(risk, RiskLevel::Critical);
    }

    #[test]
    fn test_sanctioned_addresses() {
        let mut checker = ComplianceChecker::new();
        let sanctioned_addr = "0x1111111111111111111111111111111111111111";

        checker.add_sanctioned_address(sanctioned_addr.to_string());
        assert!(checker.is_address_sanctioned(sanctioned_addr));

        let result = checker
            .check_transaction("user123", &TransactionType::Transfer, 100.0, sanctioned_addr, "US")
            .unwrap();

        match result {
            ComplianceResult::NonCompliant(_) => {}
            _ => panic!("Expected non-compliant for sanctioned address"),
        }

        checker.remove_sanctioned_address(sanctioned_addr);
        assert!(!checker.is_address_sanctioned(sanctioned_addr));
    }
}
