//! 多签transaction管理
//!
//! 提供多签transaction的创建和状态管理

use secp256k1::{ecdsa::Signature, PublicKey};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use super::config::AmountPrecision;

/// 多签transaction
#[derive(Debug, Clone)]
pub struct MultiSigTransaction {
    /// transaction ID
    pub id: String,
    /// Recipient address
    pub to_address: String,
    /// 金额
    pub amount: String,
    /// network
    pub network: String,
    /// sign集合（sign者ID → (公钥, sign)）
    pub(crate) signatures: HashMap<String, (PublicKey, Signature)>,
    /// sign阈值
    pub threshold: u8,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// 允许的sign者列表（可选）
    pub allowed_signers: Option<Vec<PublicKey>>,
    /// Nonce（防重放）
    pub nonce: Option<u64>,
    /// Chain ID（防跨链重放）
    pub chain_id: Option<u64>,
    /// 金额精度
    pub amount_precision: AmountPrecision,
}

impl MultiSigTransaction {
    /// 创建新的多签transaction
    pub fn new(
        id: String,
        to_address: String,
        amount: String,
        network: String,
        threshold: u8,
    ) -> Self {
        Self {
            id,
            to_address,
            amount,
            network,
            signatures: HashMap::new(),
            threshold,
            created_at: Utc::now(),
            allowed_signers: None,
            nonce: None,
            chain_id: None,
            amount_precision: AmountPrecision::Raw,
        }
    }

    /// fetch已收集的sign数量
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }

    /// check是否达到sign阈值
    pub fn is_complete(&self) -> bool {
        self.signature_count() >= self.threshold as usize
    }

    /// fetchsign的十六进制表示
    pub fn signatures_hex(&self) -> Vec<(String, String)> {
        self.signatures
            .iter()
            .map(|(signer_id, (_pk, sig))| {
                let compact = sig.serialize_compact();
                (signer_id.clone(), hex::encode(compact))
            })
            .collect()
    }

    /// 添加sign
    pub(crate) fn add_signature(
        &mut self,
        signer_id: String,
        pubkey: PublicKey,
        signature: Signature,
    ) -> Result<(), anyhow::Error> {
        // check是否已有此sign者的sign
        if self.signatures.contains_key(&signer_id) {
            return Err(anyhow::anyhow!("Duplicate signature from signer: {}", signer_id));
        }

        // checksign者是否在允许列表中
        if let Some(ref allowed) = self.allowed_signers {
            if !allowed.iter().any(|pk| pk == &pubkey) {
                return Err(anyhow::anyhow!("Signer not in allowed list"));
            }
        }

        self.signatures.insert(signer_id, (pubkey, signature));
        Ok(())
    }
}

/// 待处理的多签transaction（别名，向后兼容）
pub type PendingMultiSigTransaction = MultiSigTransaction;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multisig_transaction_creation() {
        let tx = MultiSigTransaction::new(
            "tx_001".to_string(),
            "0x1234".to_string(),
            "1.5".to_string(),
            "eth".to_string(),
            2,
        );

        assert_eq!(tx.id, "tx_001");
        assert_eq!(tx.threshold, 2);
        assert_eq!(tx.signature_count(), 0);
        assert!(!tx.is_complete());
    }

    #[test]
    fn test_signature_count() {
        let tx = MultiSigTransaction::new(
            "tx_002".to_string(),
            "0x5678".to_string(),
            "2.0".to_string(),
            "eth".to_string(),
            3,
        );

        assert_eq!(tx.signature_count(), 0);
    }
}

