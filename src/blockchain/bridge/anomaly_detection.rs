//! 跨链桥异常检测集成
//!
//! 为跨链桥transaction提供异常检测功能

use crate::anomaly_detection::{AnomalyDetector, AnomalyResult};
use crate::core::errors::WalletError;
use tracing::{info, warn};

/// 跨链桥异常检测扩展
pub trait BridgeAnomalyDetection {
    /// validate跨链转账
    fn validate_bridge_transfer(
        &mut self,
        from_chain: &str,
        to_chain: &str,
        to_address: &str,
        amount: f64,
    ) -> Result<(), WalletError>;

    /// check桥接transaction（不阻止）
    fn check_bridge_transaction(
        &mut self,
        from_chain: &str,
        to_chain: &str,
        to_address: &str,
        amount: f64,
    ) -> AnomalyResult;
}

impl BridgeAnomalyDetection for AnomalyDetector {
    fn validate_bridge_transfer(
        &mut self,
        from_chain: &str,
        to_chain: &str,
        to_address: &str,
        amount: f64,
    ) -> Result<(), WalletError> {
        info!(
            "🌉 [Bridge] validate跨链转账: {} -> {} (amount: {}, to: {})",
            from_chain, to_chain, amount, to_address
        );

        // 跨链桥特定check
        if !Self::is_supported_chain(from_chain) || !Self::is_supported_chain(to_chain) {
            return Err(WalletError::ValidationError(
                format!("Unsupported chain: {} or {}", from_chain, to_chain)
            ));
        }

        // 跨链桥风险更高，需要额外validate
        if amount > 100.0 {
            warn!("⚠️ 大额跨链转账 ({}), 需要额外Note", amount);
        }

        // 跨链桥transaction视为合约调用
        self.validate_transaction(
            to_address,
            amount,
            None,
            true, // 桥接是合约调用
        )
    }

    fn check_bridge_transaction(
        &mut self,
        _from_chain: &str,
        _to_chain: &str,
        to_address: &str,
        amount: f64,
    ) -> AnomalyResult {
        self.detect_transaction(
            to_address,
            amount,
            None,
            true,
        )
    }
}

impl AnomalyDetector {
    /// check是否支持该链
    fn is_supported_chain(chain: &str) -> bool {
        matches!(
            chain.to_lowercase().as_str(),
            "ethereum" | "polygon" | "bsc" | "polygon" | "arbitrum" | "optimism" | "avalanche"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomaly_detection::DetectionMode;

    #[test]
    fn test_bridge_transfer_validation() {
        let mut detector = AnomalyDetector::new();
        detector.set_mode(DetectionMode::WarnOnly);

        let result = detector.validate_bridge_transfer(
            "polygon",
            "ethereum",
            "0x1234567890123456789012345678901234567890",
            10.0,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_unsupported_chain() {
        let mut detector = AnomalyDetector::new();

        let result = detector.validate_bridge_transfer(
            "unknown_chain",
            "ethereum",
            "0x1234567890123456789012345678901234567890",
            10.0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_high_value_bridge_transfer() {
        let mut detector = AnomalyDetector::new();
        detector.set_mode(DetectionMode::WarnOnly);

        let result = detector.validate_bridge_transfer(
            "polygon",
            "ethereum",
            "0x1234567890123456789012345678901234567890",
            150.0, // 大额转账
        );

        // WarnOnly 模式应该允许
        assert!(result.is_ok());
    }
}

