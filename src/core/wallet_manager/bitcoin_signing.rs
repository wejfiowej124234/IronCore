//! Bitcointransactionsign模块
//!
//! 实现Bitcoin P2PKHtransaction的构建和sign
//!
//! ## transaction流程
//! 1. queryUTXO
//! 2. 选择UTXO（Coin Selection）
//! 3. 构建transaction（inputs + outputs）
//! 4. 计算手续费
//! 5. sign每个输入
//! 6. 序列化transaction
//! 7. 广播到network

use super::WalletManager;
use super::bitcoin_utxo::{query_utxos, select_utxos, get_recommended_fee_rate};
use crate::core::errors::WalletError;
use tracing::{debug, info};

#[cfg(feature = "bitcoin")]
impl WalletManager {
    /// signBitcointransaction
    ///
    /// # Arguments
    /// * `wallet_name` - Wallet name
    /// * `to` - Recipient address
    /// * `amount_btc` - 转账金额（BTC单位，如 "0.001"）
    /// * `password` - userPassword
    ///
    /// # Returns
    /// * 序列化的transaction（hex格式）
    pub async fn sign_bitcoin_transaction(
        &self,
        wallet_name: &str,
        to: &str,
        amount_btc: &str,
        password: &str,
    ) -> Result<String, WalletError> {
        info!("Signing Bitcoin transaction for wallet: {}", wallet_name);
        
        use bitcoin::secp256k1::{Secp256k1, Message, SecretKey};
        use bitcoin::{Transaction, TxIn, TxOut, OutPoint, Script, Witness};
        use bitcoin::{Address, Network, PublicKey as BitcoinPublicKey};
        use bitcoin::transaction::Version;
        use bitcoin::absolute::LockTime;
        use bitcoin::Sequence;
        use bitcoin::script::Builder as ScriptBuilder;
        use bitcoin::sighash::{SighashCache, EcdsaSighashType};
        use bitcoin::Amount;
        use std::str::FromStr;
        
        // Step 1: fetchwallet数据
        let wallet = self.get_wallet_by_name(wallet_name).await?
            .ok_or_else(|| WalletError::NotFoundError(
                format!("Wallet '{}' not found", wallet_name)
            ))?;
        
        // Step 2: 解密master_key并创建Private key
        let master_key = self.decrypt_master_key(&wallet, password).await?;
        
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&master_key)
            .map_err(|e| WalletError::CryptoError(format!("Invalid secret key: {}", e)))?;
        
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let bitcoin_pubkey = BitcoinPublicKey::new(public_key);
        
        // Step 3: fetchSender address
        let from_address = Address::p2pkh(&bitcoin_pubkey, Network::Bitcoin);
        let from_address_str = from_address.to_string();
        
        info!("From address: {}", from_address_str);
        
        // Step 4: queryUTXO
        let utxos = query_utxos(&from_address_str).await?;
        
        if utxos.is_empty() {
            return Err(WalletError::ValidationError(
                "No UTXOs available (balance is 0)".to_string()
            ));
        }
        
        // Step 5: 解析目标金额（BTC → satoshi）
        let amount_f64: f64 = amount_btc.parse()
            .map_err(|_| WalletError::ValidationError("Invalid amount".to_string()))?;
        let amount_satoshi = (amount_f64 * 100_000_000.0) as u64;
        
        info!("Target amount: {} BTC = {} satoshi", amount_btc, amount_satoshi);
        
        // Step 6: fetch手续费率
        let fee_rate = get_recommended_fee_rate().await?;
        
        // Step 7: 选择UTXO
        let (selected_utxos, change) = select_utxos(utxos, amount_satoshi, fee_rate)?;
        
        info!("Selected {} UTXOs, change: {} satoshi", selected_utxos.len(), change);
        
        // Step 8: 解析Recipient address
        let to_address = Address::from_str(to)
            .map_err(|e| WalletError::ValidationError(format!("Invalid address: {}", e)))?
            .assume_checked(); // bitcoin 0.31需要显式check
        
        // Step 9: 构建transaction输入
        let mut inputs = Vec::new();
        for utxo in &selected_utxos {
            let txid = bitcoin::Txid::from_str(&utxo.txid)
                .map_err(|e| WalletError::ValidationError(format!("Invalid txid: {}", e)))?;
            
            let outpoint = OutPoint {
                txid,
                vout: utxo.vout,
            };
            
            let input = TxIn {
                previous_output: outpoint,
                script_sig: Script::new().into(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            };
            
            inputs.push(input);
        }
        
        // Step 10: 构建transaction输出
        let mut outputs = Vec::new();
        
        // 输出1: 发送到目标address
        outputs.push(TxOut {
            value: Amount::from_sat(amount_satoshi),
            script_pubkey: to_address.script_pubkey(),
        });
        
        // 输出2: 找零（如果有）
        if change > 546 { // Bitcoin粉尘限制（546 satoshi）
            outputs.push(TxOut {
                value: Amount::from_sat(change),
                script_pubkey: from_address.script_pubkey(),
            });
        }
        
        // Step 11: 构建未Sign transaction
        let mut tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        };
        
        debug!("Transaction constructed: {} inputs, {} outputs", 
               tx.input.len(), tx.output.len());
        
        // Step 12: sign每个输入
        for (index, _utxo) in selected_utxos.iter().enumerate() {
            // 创建sign哈希
            let sighash_type = EcdsaSighashType::All;
            
            // 创建脚本（P2PKH）
            let script_code = from_address.script_pubkey();
            
            // 计算sign哈希
            let sighash_cache = SighashCache::new(&tx);
            let sighash = sighash_cache
                .legacy_signature_hash(index, &script_code, sighash_type.to_u32())
                .map_err(|e| WalletError::CryptoError(format!("Failed to create sighash: {}", e)))?;
            
            let message = Message::from_digest_slice(sighash.as_ref())
                .map_err(|e| WalletError::CryptoError(format!("Invalid message: {}", e)))?;
            
            // sign
            let signature = secp.sign_ecdsa(&message, &secret_key);
            
            // 创建sign脚本
            let mut sig_with_hashtype = signature.serialize_der().to_vec();
            sig_with_hashtype.push(sighash_type.to_u32() as u8);
            
            // 构建scriptSig: <signature> <pubkey>
            use bitcoin::script::PushBytesBuf;
            
            let sig_bytes = PushBytesBuf::try_from(sig_with_hashtype)
                .map_err(|e| WalletError::CryptoError(format!("Invalid signature bytes: {:?}", e)))?;
            
            let pubkey_bytes = bitcoin_pubkey.to_bytes();
            let pubkey_push = PushBytesBuf::try_from(pubkey_bytes.to_vec())
                .map_err(|e| WalletError::CryptoError(format!("Invalid pubkey bytes: {:?}", e)))?;
            
            let script_sig = ScriptBuilder::new()
                .push_slice(sig_bytes)
                .push_slice(pubkey_push)
                .into_script();
            
            tx.input[index].script_sig = script_sig.into();
            
            debug!("Signed input {}/{}", index + 1, tx.input.len());
        }
        
        // Step 13: 序列化transaction
        use bitcoin::consensus::encode::serialize;
        let serialized = serialize(&tx);
        let tx_hex = hex::encode(&serialized);
        
        info!("✅ Transaction signed successfully, size: {} bytes, hex length: {}", 
              serialized.len(), tx_hex.len());
        
        Ok(tx_hex)
    }
    
    /// 广播Bitcointransaction
    ///
    /// # Arguments
    /// * `tx_hex` - 序列化的transaction（hex格式）
    ///
    /// # Returns
    /// * transactionID
    pub async fn broadcast_bitcoin_transaction(
        &self,
        tx_hex: &str,
    ) -> Result<String, WalletError> {
        info!("Broadcasting Bitcoin transaction, size: {} bytes", tx_hex.len() / 2);
        
        // 使用Blockstream API广播
        let url = "https://blockstream.info/api/tx";
        
        let client = reqwest::Client::new();
        let response = client.post(url)
            .body(tx_hex.to_string())
            .send()
            .await
            .map_err(|e| WalletError::NetworkError(format!("Failed to broadcast: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(WalletError::NetworkError(format!(
                "Broadcast failed: {}",
                error_text
            )));
        }
        
        let txid = response.text().await
            .map_err(|e| WalletError::NetworkError(format!("Failed to read txid: {}", e)))?;
        
        info!("✅ Transaction broadcasted successfully: {}", txid);
        
        Ok(txid)
    }
}

#[cfg(test)]
mod tests {
    // 测试需要真实network，暂时跳过
    // 可以在集成测试中validate
}

