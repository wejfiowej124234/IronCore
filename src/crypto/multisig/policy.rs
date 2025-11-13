//! 多签策略管理
//!
//! 提供阈值策略和sign权限管理

use secp256k1::PublicKey;

/// 阈值策略
#[derive(Debug, Clone)]
pub struct ThresholdPolicy {
    /// 所需sign数
    pub required: u8,
    /// 总sign者数
    pub total: u8,
}

impl ThresholdPolicy {
    /// 创建新的阈值策略
    pub fn new(required: u8, total: u8) -> Result<Self, anyhow::Error> {
        if required == 0 {
            return Err(anyhow::anyhow!("Required signatures must be at least 1"));
        }
        if required > total {
            return Err(anyhow::anyhow!("Required cannot exceed total"));
        }

        Ok(Self { required, total })
    }

    /// M-of-N 多签策略
    pub fn m_of_n(m: u8, n: u8) -> Result<Self, anyhow::Error> {
        Self::new(m, n)
    }

    /// check是否满足阈值
    pub fn is_satisfied(&self, signature_count: usize) -> bool {
        signature_count >= self.required as usize
    }

    /// 计算success率（当前sign数 / 所需sign数）
    pub fn completion_rate(&self, signature_count: usize) -> f64 {
        signature_count as f64 / self.required as f64
    }
}

/// sign权限管理
pub struct SignerPermissions {
    /// 白名单sign者
    whitelist: Vec<PublicKey>,
    /// 黑名单sign者
    blacklist: Vec<PublicKey>,
}

impl SignerPermissions {
    /// 创建新的权限管理器
    pub fn new() -> Self {
        Self {
            whitelist: Vec::new(),
            blacklist: Vec::new(),
        }
    }

    /// 添加到白名单
    pub fn add_to_whitelist(&mut self, pubkey: PublicKey) {
        if !self.whitelist.contains(&pubkey) {
            self.whitelist.push(pubkey);
        }
    }

    /// 添加到黑名单
    pub fn add_to_blacklist(&mut self, pubkey: PublicKey) {
        if !self.blacklist.contains(&pubkey) {
            self.blacklist.push(pubkey);
        }
    }

    /// checksign者是否有权限
    pub fn is_authorized(&self, pubkey: &PublicKey) -> bool {
        // 黑名单优先
        if self.blacklist.contains(pubkey) {
            return false;
        }

        // 如果有白名单，必须在白名单中
        if !self.whitelist.is_empty() {
            return self.whitelist.contains(pubkey);
        }

        // 无限制
        true
    }
}

impl Default for SignerPermissions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_policy() {
        let policy = ThresholdPolicy::m_of_n(2, 3).unwrap();
        assert_eq!(policy.required, 2);
        assert_eq!(policy.total, 3);
        assert!(!policy.is_satisfied(1));
        assert!(policy.is_satisfied(2));
        assert!(policy.is_satisfied(3));
    }

    #[test]
    fn test_invalid_policy() {
        assert!(ThresholdPolicy::m_of_n(0, 3).is_err());
        assert!(ThresholdPolicy::m_of_n(4, 3).is_err());
    }

    #[test]
    fn test_completion_rate() {
        let policy = ThresholdPolicy::m_of_n(3, 5).unwrap();
        assert_eq!(policy.completion_rate(0), 0.0);
        assert_eq!(policy.completion_rate(3), 1.0);
        assert!(policy.completion_rate(2) < 1.0);
    }
}

