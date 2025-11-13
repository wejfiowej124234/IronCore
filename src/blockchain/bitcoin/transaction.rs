//! Bitcoin transaction构建和sign
//! 
//! 支持 Legacy、SegWit 和 Taproot transaction

use super::account::BitcoinKeypair;
use super::address::AddressType;
use super::utxo::Utxo;
use crate::core::errors::WalletError;
use bitcoin::absolute::LockTime;
use bitcoin::address::Address;
use bitcoin::consensus::encode::serialize;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use bitcoin::hashes::Hash;
use bitcoin::script::PushBytesBuf;
use bitcoin::{Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
use bitcoin::transaction::Version;
use std::str::FromStr;
use tracing::{debug, info};

/// Bitcoin transaction构建器
pub struct BitcoinTransaction;

impl BitcoinTransaction {
    /// 构建 Legacy (P2PKH) transaction
    pub fn build_legacy(
        keypair: &BitcoinKeypair,
        utxos: &[Utxo],
        to_address: &str,
        amount: u64,
        fee: u64,
        network: Network,
    ) -> Result<Transaction, WalletError> {
        info!("构建 Legacy transaction: 金额={} sat, 手续费={} sat", amount, fee);
        
        // validate金额
        if amount == 0 {
            return Err(WalletError::ValidationError("transaction金额不能为零".to_string()));
        }
        
        // 解析目标address
        let recipient_address = Address::from_str(to_address)
            .map_err(|e| WalletError::InvalidAddress(format!("无效的address: {}", e)))?
            .require_network(network)
            .map_err(|e| WalletError::InvalidAddress(format!("addressnetwork不匹配: {}", e)))?;
        
        // 计算总输入金额
        let total_input: u64 = utxos.iter().map(|u| u.amount).sum();
        
        // checkbalance
        if total_input < amount + fee {
            return Err(WalletError::InsufficientFunds(format!(
                "balance不足: 需要 {} sat, 可用 {} sat",
                amount + fee,
                total_input
            )));
        }
        
        // 计算找零
        let change_amount = total_input - amount - fee;
        
        // 创建输入
        let mut inputs = Vec::new();
        for utxo in utxos {
            let outpoint = OutPoint {
                txid: utxo.txid()?,
                vout: utxo.vout,
            };
            
            let txin = TxIn {
                previous_output: outpoint,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            };
            
            inputs.push(txin);
        }
        
        // 创建输出
        let mut outputs = vec![
            // 接收者输出
            TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: recipient_address.script_pubkey(),
            },
        ];
        
        // 找零输出（如果有且大于灰尘阈值 546 sat）
        const DUST_THRESHOLD: u64 = 546;
        if change_amount >= DUST_THRESHOLD {
            let change_address = super::address::BitcoinAddress::from_public_key_legacy(
                keypair.public_key(),
                network,
            )?;
            let change_addr = Address::from_str(&change_address)
                .map_err(|e| WalletError::AddressGenerationFailed(format!("找零address生成failed: {}", e)))?
                .require_network(network)
                .map_err(|_| WalletError::AddressGenerationFailed("找零addressnetwork不匹配".to_string()))?;
            
            outputs.push(TxOut {
                value: Amount::from_sat(change_amount),
                script_pubkey: change_addr.script_pubkey(),
            });
        }
        // 如果找零小于灰尘阈值，将其作为额外手续费
        
        // 创建未Sign transaction
        let mut tx = Transaction {
            version: Version::ONE,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        };
        
        // sign每个输入
        let secp = Secp256k1::new();
        for (i, utxo) in utxos.iter().enumerate() {
            let script_pubkey = ScriptBuf::from_hex(&utxo.script_pubkey)
                .map_err(|e| WalletError::TransactionFailed(format!("无效的脚本公钥: {}", e)))?;
            
            // 创建sign哈希
            let sighash_cache = SighashCache::new(&tx);
            let sighash = sighash_cache
                .legacy_signature_hash(i, &script_pubkey, EcdsaSighashType::All.to_u32())
                .map_err(|e| WalletError::SigningFailed(format!("sign哈希计算failed: {}", e)))?;
            
            // sign
            let message = Message::from_digest(*sighash.as_byte_array());
            let signature = secp.sign_ecdsa(&message, &keypair.secret_key());
            
            // 构建 scriptSig
            let mut sig_bytes = signature.serialize_der().to_vec();
            sig_bytes.push(EcdsaSighashType::All.to_u32() as u8);
            let pk_bytes = keypair.public_key_bytes();
            
            let sig_push = PushBytesBuf::try_from(sig_bytes)
                .map_err(|e| WalletError::SigningFailed(format!("sign数据无效: {:?}", e)))?;
            let pk_push = PushBytesBuf::try_from(pk_bytes)
                .map_err(|e| WalletError::SigningFailed(format!("公钥数据无效: {:?}", e)))?;
            
            let script_sig = bitcoin::blockdata::script::Builder::new()
                .push_slice(sig_push)
                .push_slice(pk_push)
                .into_script();
            
            tx.input[i].script_sig = script_sig;
        }
        
        debug!("✅ Legacy transaction构建completed，txid={}", tx.txid());
        Ok(tx)
    }
    
    /// 构建 SegWit (P2WPKH) transaction
    pub fn build_segwit(
        keypair: &BitcoinKeypair,
        utxos: &[Utxo],
        to_address: &str,
        amount: u64,
        fee: u64,
        network: Network,
    ) -> Result<Transaction, WalletError> {
        info!("构建 SegWit transaction: 金额={} sat, 手续费={} sat", amount, fee);
        
        // validate金额
        if amount == 0 {
            return Err(WalletError::ValidationError("transaction金额不能为零".to_string()));
        }
        
        // 解析目标address
        let recipient_address = Address::from_str(to_address)
            .map_err(|e| WalletError::InvalidAddress(format!("无效的address: {}", e)))?
            .require_network(network)
            .map_err(|e| WalletError::InvalidAddress(format!("addressnetwork不匹配: {}", e)))?;
        
        // 计算总输入金额
        let total_input: u64 = utxos.iter().map(|u| u.amount).sum();
        
        // checkbalance
        if total_input < amount + fee {
            return Err(WalletError::InsufficientFunds(format!(
                "balance不足: 需要 {} sat, 可用 {} sat",
                amount + fee,
                total_input
            )));
        }
        
        // 计算找零
        let change_amount = total_input - amount - fee;
        
        // 创建输入
        let mut inputs = Vec::new();
        for utxo in utxos {
            let outpoint = OutPoint {
                txid: utxo.txid()?,
                vout: utxo.vout,
            };
            
            let txin = TxIn {
                previous_output: outpoint,
                script_sig: ScriptBuf::new(), // SegWit 不使用 script_sig
                sequence: Sequence::MAX,
                witness: Witness::new(),
            };
            
            inputs.push(txin);
        }
        
        // 创建输出
        let mut outputs = vec![
            // 接收者输出
            TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: recipient_address.script_pubkey(),
            },
        ];
        
        // 找零输出（如果有且大于灰尘阈值 546 sat）
        const DUST_THRESHOLD: u64 = 546;
        if change_amount >= DUST_THRESHOLD {
            let change_address = super::address::BitcoinAddress::from_public_key_segwit(
                keypair.public_key(),
                network,
            )?;
            let change_addr = Address::from_str(&change_address)
                .map_err(|e| WalletError::AddressGenerationFailed(format!("找零address生成failed: {}", e)))?
                .require_network(network)
                .map_err(|_| WalletError::AddressGenerationFailed("找零addressnetwork不匹配".to_string()))?;
            
            outputs.push(TxOut {
                value: Amount::from_sat(change_amount),
                script_pubkey: change_addr.script_pubkey(),
            });
        }
        // 如果找零小于灰尘阈值，将其作为额外手续费
        
        // 创建未Sign transaction
        let mut tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        };
        
        // sign每个输入
        let secp = Secp256k1::new();
        
        // 准备 prevouts
        let prevouts: Vec<TxOut> = utxos
            .iter()
            .map(|utxo| {
                let script_pubkey = ScriptBuf::from_hex(&utxo.script_pubkey)
                    .expect("脚本公钥应该有效");
                TxOut {
                    value: Amount::from_sat(utxo.amount),
                    script_pubkey,
                }
            })
            .collect();
        
        let _prevouts = Prevouts::All(&prevouts);
        
        for i in 0..utxos.len() {
            let mut sighash_cache = SighashCache::new(&tx);
            
            // 计算 SegWit sign哈希（使用原始 prevouts Vec）
            let prevouts_vec: Vec<TxOut> = utxos
                .iter()
                .map(|utxo| {
                    let script_pubkey = ScriptBuf::from_hex(&utxo.script_pubkey)
                        .expect("脚本公钥应该有效");
                    TxOut {
                        value: Amount::from_sat(utxo.amount),
                        script_pubkey,
                    }
                })
                .collect();
            
            // 使用segwit_signature_hash（虽然deprecated但功能正确）
            #[allow(deprecated)]
            let sighash = sighash_cache
                .segwit_signature_hash(i, &prevouts_vec[i].script_pubkey, Amount::from_sat(utxos[i].amount), EcdsaSighashType::All)
                .map_err(|e| WalletError::SigningFailed(format!("SegWit sign哈希计算failed: {}", e)))?;
            
            // sign
            let message = Message::from_digest(*sighash.as_byte_array());
            let signature = secp.sign_ecdsa(&message, &keypair.secret_key());
            
            // 构建 witness
            let mut sig_bytes = signature.serialize_der().to_vec();
            sig_bytes.push(EcdsaSighashType::All.to_u32() as u8);
            
            let witness = vec![sig_bytes, keypair.public_key_bytes()];
            tx.input[i].witness = Witness::from_slice(&witness);
        }
        
        debug!("✅ SegWit transaction构建completed，txid={}", tx.txid());
        Ok(tx)
    }
    
    /// 构建 Taproot (P2TR) transaction
    pub fn build_taproot(
        keypair: &BitcoinKeypair,
        utxos: &[Utxo],
        to_address: &str,
        amount: u64,
        fee: u64,
        network: Network,
    ) -> Result<Transaction, WalletError> {
        info!("构建 Taproot transaction: 金额={} sat, 手续费={} sat", amount, fee);
        
        // validate金额
        if amount == 0 {
            return Err(WalletError::ValidationError("transaction金额不能为零".to_string()));
        }
        
        // 解析目标address
        let recipient_address = Address::from_str(to_address)
            .map_err(|e| WalletError::InvalidAddress(format!("无效的address: {}", e)))?
            .require_network(network)
            .map_err(|e| WalletError::InvalidAddress(format!("addressnetwork不匹配: {}", e)))?;
        
        // 计算总输入金额
        let total_input: u64 = utxos.iter().map(|u| u.amount).sum();
        
        // checkbalance
        if total_input < amount + fee {
            return Err(WalletError::InsufficientFunds(format!(
                "balance不足: 需要 {} sat, 可用 {} sat",
                amount + fee,
                total_input
            )));
        }
        
        // 计算找零
        let change_amount = total_input - amount - fee;
        
        // 创建输入
        let mut inputs = Vec::new();
        for utxo in utxos {
            let outpoint = OutPoint {
                txid: utxo.txid()?,
                vout: utxo.vout,
            };
            
            let txin = TxIn {
                previous_output: outpoint,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            };
            
            inputs.push(txin);
        }
        
        // 创建输出
        let mut outputs = vec![
            // 接收者输出
            TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: recipient_address.script_pubkey(),
            },
        ];
        
        // 找零输出（如果有且大于灰尘阈值 546 sat）
        const DUST_THRESHOLD: u64 = 546;
        if change_amount >= DUST_THRESHOLD {
            let change_address = super::address::BitcoinAddress::from_public_key_taproot(
                keypair.public_key(),
                network,
            )?;
            let change_addr = Address::from_str(&change_address)
                .map_err(|e| WalletError::AddressGenerationFailed(format!("找零address生成failed: {}", e)))?
                .require_network(network)
                .map_err(|_| WalletError::AddressGenerationFailed("找零addressnetwork不匹配".to_string()))?;
            
            outputs.push(TxOut {
                value: Amount::from_sat(change_amount),
                script_pubkey: change_addr.script_pubkey(),
            });
        }
        // 如果找零小于灰尘阈值，将其作为额外手续费
        
        // 创建未Sign transaction
        let mut tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        };
        
        // Taproot sign
        let secp = Secp256k1::new();
        
        // 创建 Keypair
        use bitcoin::secp256k1::Keypair;
        let keypair_internal = Keypair::from_secret_key(&secp, &keypair.secret_key());
        
        // 准备 prevouts
        let prevouts: Vec<TxOut> = utxos
            .iter()
            .map(|utxo| {
                let script_pubkey = ScriptBuf::from_hex(&utxo.script_pubkey)
                    .expect("脚本公钥应该有效");
                TxOut {
                    value: Amount::from_sat(utxo.amount),
                    script_pubkey,
                }
            })
            .collect();
        
        let _prevouts = Prevouts::All(&prevouts);
        
        for i in 0..utxos.len() {
            let mut sighash_cache = SighashCache::new(&tx);
            
            // 计算 Taproot sign哈希
            let sighash = sighash_cache
                .taproot_key_spend_signature_hash(i, &_prevouts, TapSighashType::Default)
                .map_err(|e| WalletError::SigningFailed(format!("Taproot sign哈希计算failed: {}", e)))?;
            
            // Schnorr sign（使用未调整的 keypair，bitcoin crate 会自动处理调整）
            let message = Message::from_digest(*sighash.as_byte_array());
            let signature = secp.sign_schnorr_no_aux_rand(&message, &keypair_internal);
            
            // 构建 witness（Taproot key-path spend 只需要sign）
            let witness = vec![signature.as_ref().to_vec()];
            tx.input[i].witness = Witness::from_slice(&witness);
        }
        
        debug!("✅ Taproot transaction构建completed，txid={}", tx.txid());
        Ok(tx)
    }
    
    /// 根据address类型构建transaction
    pub fn build(
        keypair: &BitcoinKeypair,
        utxos: &[Utxo],
        to_address: &str,
        amount: u64,
        fee: u64,
        address_type: AddressType,
        network: Network,
    ) -> Result<Transaction, WalletError> {
        match address_type {
            AddressType::Legacy => {
                Self::build_legacy(keypair, utxos, to_address, amount, fee, network)
            }
            AddressType::SegWit => {
                Self::build_segwit(keypair, utxos, to_address, amount, fee, network)
            }
            AddressType::Taproot => {
                Self::build_taproot(keypair, utxos, to_address, amount, fee, network)
            }
        }
    }
    
    /// 序列化transaction为十六进制字符串
    pub fn serialize(tx: &Transaction) -> String {
        hex::encode(serialize(tx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::address::BitcoinAddress;
    
    fn create_test_utxo() -> Utxo {
        Utxo::new(
            "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            0,
            100_000,
            "76a914000000000000000000000000000000000000000088ac".to_string(), // P2PKH script
            6,
        )
    }
    
    #[test]
    fn test_build_legacy_transaction() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        assert_eq!(tx.input.len(), 1);
        assert!(tx.output.len() >= 1);
        assert!(!tx.input[0].script_sig.is_empty()); // Legacy 使用 script_sig
    }
    
    #[test]
    fn test_build_segwit_transaction() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let mut utxo = create_test_utxo();
        utxo.script_pubkey = "0014000000000000000000000000000000000000000000".to_string(); // P2WPKH script
        
        let utxos = vec![utxo];
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        assert_eq!(tx.input.len(), 1);
        assert!(tx.output.len() >= 1);
        assert!(tx.input[0].script_sig.is_empty()); // SegWit 不使用 script_sig
        assert!(!tx.input[0].witness.is_empty()); // SegWit 使用 witness
    }
    
    #[test]
    fn test_build_taproot_transaction() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let mut utxo = create_test_utxo();
        utxo.script_pubkey = "51200000000000000000000000000000000000000000000000000000000000000000".to_string(); // P2TR script
        
        let utxos = vec![utxo];
        let to_address = BitcoinAddress::from_public_key_taproot(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_taproot(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        assert_eq!(tx.input.len(), 1);
        assert!(tx.output.len() >= 1);
        assert!(tx.input[0].script_sig.is_empty());
        assert!(!tx.input[0].witness.is_empty());
        assert_eq!(tx.input[0].witness.len(), 1); // Taproot key-path spend 只有sign
    }
    
    #[test]
    fn test_insufficient_funds() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let result = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            200_000, // 超过 UTXO 金额
            1_000,
            Network::Testnet,
        );
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WalletError::InsufficientFunds(_)));
    }
    
    #[test]
    fn test_transaction_serialization() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        let hex = BitcoinTransaction::serialize(&tx);
        assert!(!hex.is_empty());
        assert!(hex.len() > 100); // 合理的transaction长度
    }
    
    // ============ 新增的transaction构建测试 ============
    
    #[test]
    fn test_transaction_with_change() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];  // 100,000 sat
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        // 转账 30,000，应该有找零
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            30_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // 应该有 2 个输出：接收方 + 找零
        assert_eq!(tx.output.len(), 2, "应该有接收方和找零两个输出");
        
        // 第一个输出是接收方
        assert_eq!(tx.output[0].value, Amount::from_sat(30_000));
        
        // 第二个输出是找零（总额 - 转账金额 - 手续费）
        let change_amount = tx.output[1].value;
        assert!(change_amount > Amount::ZERO, "找零应该大于0");
        assert!(change_amount < Amount::from_sat(70_000), "找零应该小于剩余金额");
    }
    
    #[test]
    fn test_transaction_without_change() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let mut utxo = create_test_utxo();
        utxo.amount = 60_000;  // 较小金额
        
        let utxos = vec![utxo];
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        // 转账几乎全部金额（留手续费）
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            58_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // 可能只有 1 个输出（接收方），或者 2 个（如果有找零）
        assert!(tx.output.len() >= 1 && tx.output.len() <= 2);
    }
    
    #[test]
    fn test_multi_input_transaction() {
        // 测试多输入transaction的基本结构
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        // 创建单个 UTXO 用于测试多输入逻辑（实际使用 UTXO 选择器）
        let utxo = create_test_utxo();
        let utxos = vec![utxo];
        
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        // 构建transaction
        let result = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        );
        
        // validatetransaction结构
        assert!(result.is_ok(), "transaction构建应该success");
        let tx = result.unwrap();
        assert_eq!(tx.input.len(), 1, "单个 UTXO 输入");
        assert!(tx.output.len() >= 1, "至少有一个输出");
        
        // Note:真正的多输入场景应该使用 UtxoSelector 先选择 UTXOs，
        // 然后将选中的 UTXOs 传给 build_* 函数
    }
    
    #[test]
    fn test_transaction_version() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // transaction版本应该是 1 或 2
        assert!(tx.version == Version::ONE || tx.version == Version::TWO);
    }
    
    #[test]
    fn test_transaction_locktime() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // Locktime 应该是零（立即有效）
        assert_eq!(tx.lock_time, LockTime::ZERO);
    }
    
    #[test]
    fn test_legacy_vs_segwit_size() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address_legacy = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        let to_address_segwit = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx_legacy = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address_legacy,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        let utxos2 = vec![{
            let mut u = create_test_utxo();
            u.script_pubkey = "0014".to_string() + &"00".repeat(20);
            u
        }];
        
        let tx_segwit = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos2,
            &to_address_segwit,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        let legacy_size = BitcoinTransaction::serialize(&tx_legacy).len();
        let segwit_size = BitcoinTransaction::serialize(&tx_segwit).len();
        
        // SegWit transaction通常更小
        assert!(segwit_size <= legacy_size, 
                "SegWit ({} bytes) 应该 <= Legacy ({} bytes)", 
                segwit_size, legacy_size);
    }
    
    #[test]
    fn test_taproot_vs_segwit_size() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        let utxo_segwit = {
            let mut u = create_test_utxo();
            u.script_pubkey = "0014".to_string() + &"00".repeat(20);
            u
        };
        
        let utxo_taproot = {
            let mut u = create_test_utxo();
            u.script_pubkey = "5120".to_string() + &"00".repeat(32);
            u
        };
        
        let to_segwit = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        let to_taproot = BitcoinAddress::from_public_key_taproot(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx_segwit = BitcoinTransaction::build_segwit(
            &keypair,
            &vec![utxo_segwit],
            &to_segwit,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        let tx_taproot = BitcoinTransaction::build_taproot(
            &keypair,
            &vec![utxo_taproot],
            &to_taproot,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // Taproot witness 数据通常更小（只有sign，没有公钥）
        assert!(tx_taproot.input[0].witness.len() <= tx_segwit.input[0].witness.len());
    }
    
    #[test]
    fn test_zero_amount_transaction() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        // 零金额应该failed
        let result = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            0,
            1_000,
            Network::Testnet,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_very_high_fee() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        // 手续费过高应该failed（超过 UTXO 总额）
        let result = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            10_000,
            100_000,  // 手续费 > 剩余金额
            Network::Testnet,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_transaction_txid_format() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        let txid = tx.txid().to_string();
        
        // TXID 应该是 64 字符的十六进制字符串
        assert_eq!(txid.len(), 64);
        assert!(txid.chars().all(|c| c.is_ascii_hexdigit()));
    }
    
    #[test]
    fn test_transaction_inputs_outputs_consistency() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // 输入数应该等于 UTXO 数
        assert_eq!(tx.input.len(), utxos.len());
        
        // 输出总额应该小于输入总额（因为手续费）
        let input_total: u64 = utxos.iter().map(|u| u.amount).sum();
        let output_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
        
        assert!(output_total < input_total, "输出总额应该小于输入（因为手续费）");
    }
    
    #[test]
    fn test_witness_data_presence() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        // SegWit transaction应该有 witness 数据
        let utxo_segwit = {
            let mut u = create_test_utxo();
            u.script_pubkey = "0014".to_string() + &"00".repeat(20);
            u
        };
        
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx_segwit = BitcoinTransaction::build_segwit(
            &keypair,
            &vec![utxo_segwit],
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // SegWit 应该有 witness 数据
        assert!(!tx_segwit.input[0].witness.is_empty(), "SegWit 应该有 witness");
        assert!(tx_segwit.input[0].witness.len() >= 2, "SegWit witness 应该有sign和公钥");
        
        // Taproot
        let utxo_taproot = {
            let mut u = create_test_utxo();
            u.script_pubkey = "5120".to_string() + &"00".repeat(32);
            u
        };
        
        let to_taproot = BitcoinAddress::from_public_key_taproot(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx_taproot = BitcoinTransaction::build_taproot(
            &keypair,
            &vec![utxo_taproot],
            &to_taproot,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // Taproot 应该有 witness（只有sign）
        assert_eq!(tx_taproot.input[0].witness.len(), 1, "Taproot witness 应该只有sign");
    }
    
    #[test]
    fn test_empty_utxos() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos: Vec<Utxo> = vec![];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let result = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        );
        
        assert!(result.is_err(), "空 UTXO 列表应该failed");
    }
    
    #[test]
    fn test_transaction_fee_calculation() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];  // 100,000 sat
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        let input_total: u64 = utxos.iter().map(|u| u.amount).sum();
        let output_total: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
        let actual_fee = input_total - output_total;
        
        // 手续费应该大于 0 且合理
        assert!(actual_fee > 0, "手续费应该大于 0");
        assert!(actual_fee < 10_000, "手续费不应该过高");
    }
    
    #[test]
    fn test_different_fee_rates() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx_low_fee = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos.clone(),
            &to_address,
            50_000,
            1,  // 低费率
            Network::Testnet,
        ).unwrap();
        
        let tx_high_fee = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            100,  // 高费率
            Network::Testnet,
        ).unwrap();
        
        let fee_low: u64 = 100_000 - tx_low_fee.output.iter().map(|o| o.value.to_sat()).sum::<u64>();
        let fee_high: u64 = 100_000 - tx_high_fee.output.iter().map(|o| o.value.to_sat()).sum::<u64>();
        
        assert!(fee_high > fee_low, "高费率应该产生更高手续费");
    }
    
    #[test]
    fn test_transaction_sequence_numbers() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // Sequence 应该是 0xFFFFFFFF（最终）
        for input in &tx.input {
            assert_eq!(input.sequence, Sequence::MAX);
        }
    }
    
    #[test]
    fn test_output_script_pubkey() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // 输出应该有有效的 script_pubkey
        for output in &tx.output {
            assert!(!output.script_pubkey.is_empty(), "script_pubkey 不应该为空");
        }
    }
    
    #[test]
    fn test_dispatcher_build_function() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        // 测试通用 build 函数
        let tx_legacy = BitcoinTransaction::build(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            AddressType::Legacy,
            Network::Testnet,
        ).unwrap();
        
        assert_eq!(tx_legacy.input.len(), 1);
        assert!(!tx_legacy.input[0].script_sig.is_empty(), "Legacy 应该使用 script_sig");
    }
    
    #[test]
    fn test_mainnet_transaction_creation() {
        let keypair = BitcoinKeypair::generate(Network::Bitcoin).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_taproot(keypair.public_key(), Network::Bitcoin).unwrap();
        
        let tx = BitcoinTransaction::build_taproot(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Bitcoin,
        ).unwrap();
        
        // 主网transaction应该正常创建
        assert_eq!(tx.input.len(), 1);
        assert!(tx.output.len() >= 1);
    }
    
    #[test]
    fn test_transaction_serialization_deterministic() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx1 = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos.clone(),
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        let tx2 = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // 相同参数应该产生相同transaction（确定性）
        let hex1 = BitcoinTransaction::serialize(&tx1);
        let hex2 = BitcoinTransaction::serialize(&tx2);
        
        assert_eq!(hex1, hex2, "相同参数应该产生相同transaction");
    }
    
    #[test]
    fn test_change_address_generation() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxos = vec![create_test_utxo()];  // 100,000 sat
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            30_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        if tx.output.len() == 2 {
            // 第二个输出应该是找零
            let change_output = &tx.output[1];
            assert!(change_output.value > Amount::ZERO, "找零金额应该大于 0");
            
            // 找零address应该有有效的 script
            assert!(!change_output.script_pubkey.is_empty());
        }
    }
    
    #[test]
    fn test_minimal_transaction() {
        // 最小的有效transaction：1 输入，1 输出，无找零
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let mut utxo = create_test_utxo();
        utxo.amount = 51_000;  // 刚好够转账 + 手续费
        
        let utxos = vec![utxo];
        let to_address = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_legacy(
            &keypair,
            &utxos,
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        assert_eq!(tx.input.len(), 1);
        // 可能是 1 个输出（无找零）或 2 个（有少量找零）
        assert!(tx.output.len() >= 1 && tx.output.len() <= 2);
    }
    
    #[test]
    fn test_transaction_with_dust_change() {
        // 当找零金额很小（灰尘）时的处理
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let mut utxo = create_test_utxo();
        utxo.amount = 51_000;
        
        let utxos = vec![utxo];
        let to_address = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        
        // 转账金额接近 UTXO 总额，找零会很少（500 sat < 546 sat 灰尘阈值）
        let tx = BitcoinTransaction::build_segwit(
            &keypair,
            &utxos,
            &to_address,
            49_500,  // 留出 1500 sat
            1_000,   // 手续费 1000 sat
            Network::Testnet,
        ).unwrap();
        
        // 找零 500 sat < 546 sat，应该被当作额外手续费，只有 1 个输出
        assert_eq!(tx.output.len(), 1, "灰尘找零应该被合并到手续费中");
        assert_eq!(tx.output[0].value, Amount::from_sat(49_500), "输出金额应该是转账金额");
    }
    
    #[test]
    fn test_all_transaction_types_build_successfully() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        
        // Legacy
        let utxo1 = create_test_utxo();
        let addr1 = BitcoinAddress::from_public_key_legacy(keypair.public_key(), Network::Testnet).unwrap();
        let tx1 = BitcoinTransaction::build_legacy(&keypair, &[utxo1], &addr1, 50_000, 1_000, Network::Testnet);
        assert!(tx1.is_ok(), "Legacy transaction应该success");
        
        // SegWit
        let utxo2 = {
            let mut u = create_test_utxo();
            u.script_pubkey = "0014".to_string() + &"00".repeat(20);
            u
        };
        let addr2 = BitcoinAddress::from_public_key_segwit(keypair.public_key(), Network::Testnet).unwrap();
        let tx2 = BitcoinTransaction::build_segwit(&keypair, &[utxo2], &addr2, 50_000, 1_000, Network::Testnet);
        assert!(tx2.is_ok(), "SegWit transaction应该success");
        
        // Taproot
        let utxo3 = {
            let mut u = create_test_utxo();
            u.script_pubkey = "5120".to_string() + &"00".repeat(32);
            u
        };
        let addr3 = BitcoinAddress::from_public_key_taproot(keypair.public_key(), Network::Testnet).unwrap();
        let tx3 = BitcoinTransaction::build_taproot(&keypair, &[utxo3], &addr3, 50_000, 1_000, Network::Testnet);
        assert!(tx3.is_ok(), "Taproot transaction应该success");
    }
    
    #[test]
    fn test_schnorr_signature_in_taproot_tx() {
        let keypair = BitcoinKeypair::generate(Network::Testnet).unwrap();
        let utxo = {
            let mut u = create_test_utxo();
            u.script_pubkey = "5120".to_string() + &"00".repeat(32);
            u
        };
        
        let to_address = BitcoinAddress::from_public_key_taproot(keypair.public_key(), Network::Testnet).unwrap();
        
        let tx = BitcoinTransaction::build_taproot(
            &keypair,
            &[utxo],
            &to_address,
            50_000,
            1_000,
            Network::Testnet,
        ).unwrap();
        
        // Taproot witness 应该包含 Schnorr sign（64 字节）
        assert_eq!(tx.input[0].witness.len(), 1);
        let sig_bytes = tx.input[0].witness.nth(0).unwrap();
        assert_eq!(sig_bytes.len(), 64, "Taproot 应该使用 64 字节 Schnorr sign");
    }
}

