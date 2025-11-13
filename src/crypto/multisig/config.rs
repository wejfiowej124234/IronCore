//! 多签配置和策略
//!
//! 定义多sign的配置、精度和策略类型

use secp256k1::PublicKey;

#[cfg(test)]
use secp256k1::SecretKey;

/// 金额精度类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountPrecision {
    /// 原始输入（可能包含小数）
    Raw,
    /// 最小单位整数（如 wei/lamports）
    Minimal,
}

/// 多签配置
#[derive(Debug, Clone)]
pub struct MultiSigConfig {
    /// sign阈值
    pub threshold: u8,
    /// 总sign者数量
    pub total_signers: u8,
    /// sign者公钥列表
    pub signers: Vec<PublicKey>,
}

impl MultiSigConfig {
    /// 创建新的多签配置
    ///
    /// # Arguments
    /// * `threshold` - 所需sign数量
    /// * `signers` - sign者公钥列表
    ///
    /// # Returns
    /// 新的 MultiSigConfig 实例
    pub fn new(threshold: u8, signers: Vec<PublicKey>) -> Self {
        Self {
            threshold,
            total_signers: signers.len() as u8,
            signers,
        }
    }

    /// validate配置有效性
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.threshold == 0 {
            return Err(anyhow::anyhow!("Threshold must be at least 1"));
        }
        if self.threshold > self.total_signers {
            return Err(anyhow::anyhow!(
                "Threshold ({}) cannot exceed total signers ({})",
                self.threshold,
                self.total_signers
            ));
        }
        if self.signers.is_empty() {
            return Err(anyhow::anyhow!("Signers list cannot be empty"));
        }
        Ok(())
    }

    /// check公钥是否是授权sign者
    pub fn is_authorized_signer(&self, pubkey: &PublicKey) -> bool {
        self.signers.iter().any(|s| s == pubkey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::Secp256k1;

    #[test]
    fn test_multisig_config_validation() {
       let secp = Secp256k1::new();
       let mut signers = Vec::new();
       
       for _ in 0..3 {
           use rand::RngCore;
           let mut secret_bytes = [0u8; 32];
           rand::rngs::OsRng.fill_bytes(&mut secret_bytes);
           let secret_key = SecretKey::from_slice(&secret_bytes).expect("32 bytes");
           let pubkey = PublicKey::from_secret_key(&secp, &secret_key);
           signers.push(pubkey);
       }

        let config = MultiSigConfig::new(2, signers);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_threshold() {
        let secp = Secp256k1::new();
        use rand::RngCore;
        let mut secret_bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut secret_bytes);
        let secret_key = SecretKey::from_slice(&secret_bytes).expect("32 bytes");
        let pubkey = PublicKey::from_secret_key(&secp, &secret_key);
        
        let config = MultiSigConfig::new(2, vec![pubkey]);
        assert!(config.validate().is_err());
    }
}

