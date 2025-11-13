//! 多签sign管理
//!
//! 提供sign收集、validate和执行功能

use super::{
    config::AmountPrecision,
    transaction::MultiSigTransaction,
};
use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1};
use std::collections::HashMap;
use tracing::info;
use anyhow::Result;

/// 多sign管理器
pub struct MultiSignature {
    /// 待处理的transaction
    pending_transactions: HashMap<String, MultiSigTransaction>,
}

impl MultiSignature {
    /// 创建新的多sign管理器
    pub fn new(threshold: u8) -> Self {
        info!("🔐 Initializing Multi-Signature manager with threshold: {}", threshold);
        Self {
            pending_transactions: HashMap::new(),
        }
    }

    /// 创建多签transaction
    ///
    /// # Arguments
    /// * `to_address` - Recipient address
    /// * `amount` - 金额
    /// * `network` - network
    /// * `allowed_signers` - 允许的sign者列表（可选）
    /// * `threshold` - sign阈值（可选，使用 config 中的值）
    ///
    /// # Returns
    /// transaction ID
    pub fn create_transaction(
        &mut self,
        to_address: &str,
        amount: &str,
        network: &str,
        allowed_signers: Option<Vec<PublicKey>>,
        threshold: Option<u8>,
    ) -> Result<String> {
        let tx_id = format!("multisig_{}", uuid::Uuid::new_v4());
        let threshold_value = threshold.unwrap_or(2);

        let mut tx = MultiSigTransaction::new(
            tx_id.clone(),
            to_address.to_string(),
            amount.to_string(),
            network.to_string(),
            threshold_value,
        );

        tx.allowed_signers = allowed_signers;

        info!("📝 Created multi-sig transaction: {}", tx_id);
        self.pending_transactions.insert(tx_id.clone(), tx);

        Ok(tx_id)
    }

    /// 为transactionsign
    ///
    /// # Arguments
    /// * `tx_id` - transaction ID
    /// * `signer_pubkey` - sign者公钥
    /// * `signature` - sign
    ///
    /// # Returns
    /// 是否达到阈值（可以执行）
    pub fn sign_transaction(
        &mut self,
        tx_id: &str,
        signer_pubkey: &PublicKey,
        signature: &Signature,
    ) -> Result<bool> {
        let transaction = self
            .pending_transactions
            .get(tx_id)
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", tx_id))?;

        // validate nonce 和 chain_id 已设置
        if transaction.nonce.is_none() || transaction.chain_id.is_none() {
            return Err(anyhow::anyhow!(
                "nonce and chain_id must be set before signing"
            ));
        }

        // validate金额精度已设置为 Minimal
        if transaction.amount_precision != AmountPrecision::Minimal {
            return Err(anyhow::anyhow!(
                "amount_precision must be Minimal before signing"
            ));
        }

        // 构建规范消息
        let message = Self::build_canonical_message(transaction)?;

        // validatesign
        let secp = Secp256k1::verification_only();
        if secp.verify_ecdsa(&message, signature, signer_pubkey).is_err() {
            return Err(anyhow::anyhow!("Invalid signature"));
        }

        // checksign者是否在允许列表中
        if let Some(allowed) = &transaction.allowed_signers {
            let signer_hex = format!("{:x}", signer_pubkey);
            let allowed_hex: Vec<String> = allowed.iter().map(|pk| format!("{:x}", pk)).collect();
            if !allowed_hex.iter().any(|s| s == &signer_hex) {
                return Err(anyhow::anyhow!("Signer not in allowed list"));
            }
        }

        // 添加sign
        let transaction = self
            .pending_transactions
            .get_mut(tx_id)
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", tx_id))?;

        let signer_id = format!("{:x}", signer_pubkey);
        transaction.add_signature(signer_id, *signer_pubkey, *signature)?;

        let is_complete = transaction.is_complete();

        if is_complete {
            info!("✅ Multi-sig transaction {} is ready ({}/{} signatures)",
                  tx_id, transaction.signature_count(), transaction.threshold);
        } else {
            info!("📝 Multi-sig transaction {} signed ({}/{} signatures)",
                  tx_id, transaction.signature_count(), transaction.threshold);
        }

        Ok(is_complete)
    }

    /// 构建规范消息用于sign
    fn build_canonical_message(tx: &MultiSigTransaction) -> Result<Message> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(b"MULTISIG_TX_V1");
        hasher.update(tx.id.as_bytes());
        hasher.update(tx.to_address.as_bytes());
        hasher.update(tx.amount.as_bytes());
        hasher.update(tx.network.as_bytes());
        hasher.update([tx.threshold]);
        
        if let Some(nonce) = tx.nonce {
            hasher.update(nonce.to_le_bytes());
        }
        if let Some(chain_id) = tx.chain_id {
            hasher.update(chain_id.to_le_bytes());
        }

        let hash = hasher.finalize();
        Message::from_slice(&hash)
            .map_err(|e| anyhow::anyhow!("Failed to create message: {}", e))
    }

    /// 设置 nonce 和 chain ID
    pub fn set_nonce_and_chain_id(
        &mut self,
        tx_id: &str,
        nonce: u64,
        chain_id: u64,
    ) -> Result<()> {
        let tx = self
            .pending_transactions
            .get_mut(tx_id)
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", tx_id))?;
        
        if tx.nonce.is_some() || tx.chain_id.is_some() {
            return Err(anyhow::anyhow!("nonce/chain_id already set (immutable)"));
        }
        
        tx.nonce = Some(nonce);
        tx.chain_id = Some(chain_id);
        Ok(())
    }

    /// 设置金额精度为最小单位
    pub fn set_amount_precision_minimal(&mut self, tx_id: &str) -> Result<()> {
        let tx = self
            .pending_transactions
            .get_mut(tx_id)
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", tx_id))?;
        
        tx.amount_precision = AmountPrecision::Minimal;
        Ok(())
    }

    /// fetchtransaction
    pub fn get_transaction(&self, tx_id: &str) -> Option<&MultiSigTransaction> {
        self.pending_transactions.get(tx_id)
    }

    /// 执行transaction
    pub fn execute_transaction(&mut self, tx_id: &str) -> Result<()> {
        let tx = self
            .pending_transactions
            .get(tx_id)
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", tx_id))?;

        if !tx.is_complete() {
            return Err(anyhow::anyhow!(
                "Not enough signatures: {}/{}",
                tx.signature_count(),
                tx.threshold
            ));
        }

        info!("✅ Executing multi-sig transaction: {}", tx_id);
        
        // 实际执行逻辑应该在这里
        // 1. 构建最终transaction
        // 2. 广播到network
        
        // 移除已执行的transaction
        self.pending_transactions.remove(tx_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transaction() {
        let mut manager = MultiSignature::new(2);
        let tx_id = manager
            .create_transaction("0x1234", "1.5", "eth", None, None)
            .unwrap();

        assert!(tx_id.starts_with("multisig_"));
        assert!(manager.get_transaction(&tx_id).is_some());
    }
}

