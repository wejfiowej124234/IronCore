//! Bitcoin UTXO管理模块
//!
//! 实现UTXO（Unspent Transaction Output）的query、选择和管理
//!
//! ## 核心功能
//! - queryaddress的所有UTXO
//! - Coin Selection（选择最优UTXO组合）
//! - UTXO缓存管理
//! - 手续费估算

use crate::core::errors::WalletError;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// UTXO结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    /// transactionID
    pub txid: String,
    /// 输出索引
    pub vout: u32,
    /// 金额（satoshi）
    pub value: u64,
    /// 脚本公钥
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_pubkey: Option<String>,
    /// 确认数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<u32>,
}

/// Blockstream API返回的UTXO格式
#[derive(Debug, Deserialize)]
struct BlockstreamUTXO {
    txid: String,
    vout: u32,
    status: UTXOStatus,
    value: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UTXOStatus {
    confirmed: bool,
    #[serde(default)]
    block_height: Option<u64>,
}

/// fromBlockstream APIqueryUTXO
///
/// # Arguments
/// * `address` - Bitcoinaddress
///
/// # Returns
/// * `Vec<UTXO>` - UTXO列表
pub async fn query_utxos(address: &str) -> Result<Vec<UTXO>, WalletError> {
    info!("Querying UTXOs for address: {}", address);
    
    let url = format!("https://blockstream.info/api/address/{}/utxo", address);
    
    let response = reqwest::get(&url).await
        .map_err(|e| WalletError::NetworkError(format!("Failed to query UTXOs: {}", e)))?;
    
    if !response.status().is_success() {
        return Err(WalletError::NetworkError(format!(
            "Blockstream API returned error: {}",
            response.status()
        )));
    }
    
    let blockstream_utxos: Vec<BlockstreamUTXO> = response.json().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse UTXO response: {}", e)))?;
    
    // 转换为内部格式，只选择已确认的UTXO
    let utxos: Vec<UTXO> = blockstream_utxos
        .into_iter()
        .filter(|u| u.status.confirmed)
        .map(|u| UTXO {
            txid: u.txid,
            vout: u.vout,
            value: u.value,
            script_pubkey: None,
            confirmations: None,
        })
        .collect();
    
    info!("✅ Found {} confirmed UTXOs, total value: {} satoshi", 
          utxos.len(), 
          utxos.iter().map(|u| u.value).sum::<u64>());
    
    Ok(utxos)
}

/// Coin Selection - 选择UTXO以满足目标金额
///
/// # 算法: 贪心算法
/// 1. 按金额from大到小排序
/// 2. 依次选择直到满足目标金额
/// 3. 计算找零
///
/// # Arguments
/// * `utxos` - 可用的UTXO列表
/// * `target_amount` - 目标金额（satoshi）
/// * `fee_per_byte` - 手续费率（satoshi/byte）
///
/// # Returns
/// * `(selected_utxos, change_amount)` - 选中的UTXO和找零金额
pub fn select_utxos(
    mut utxos: Vec<UTXO>,
    target_amount: u64,
    fee_per_byte: u64,
) -> Result<(Vec<UTXO>, u64), WalletError> {
    debug!("Selecting UTXOs for target: {} satoshi, fee rate: {} sat/byte", 
           target_amount, fee_per_byte);
    
    if utxos.is_empty() {
        return Err(WalletError::ValidationError("No UTXOs available".to_string()));
    }
    
    // 按金额from大到小排序（贪心策略）
    utxos.sort_by(|a, b| b.value.cmp(&a.value));
    
    let mut selected = Vec::new();
    let mut total_value = 0u64;
    
    // 估算transaction大小
    // 基础大小: 10 bytes (version + locktime等)
    // 每个输入: ~148 bytes (txid + vout + script + sequence)
    // 每个输出: ~34 bytes (value + script)
    let base_size = 10;
    let input_size = 148;
    let output_size = 34;
    
    for utxo in utxos {
        selected.push(utxo.clone());
        total_value += utxo.value;
        
        // 计算当前transaction大小和手续费
        let tx_size = base_size + (selected.len() * input_size) + (2 * output_size); // 2个输出（接收+找零）
        let fee = (tx_size as u64) * fee_per_byte;
        
        // check是否满足：total_value >= target_amount + fee
        if total_value >= target_amount + fee {
            let change = total_value - target_amount - fee;
            
            info!("✅ Selected {} UTXOs, total: {}, target: {}, fee: {}, change: {}",
                  selected.len(), total_value, target_amount, fee, change);
            
            return Ok((selected, change));
        }
    }
    
    // balance不足
    Err(WalletError::ValidationError(format!(
        "Insufficient balance: have {} satoshi, need {} + fees",
        total_value, target_amount
    )))
}

/// 估算Bitcointransaction手续费
///
/// # Arguments
/// * `input_count` - 输入数量
/// * `output_count` - 输出数量
/// * `fee_per_byte` - 费率（satoshi/byte）
///
/// # Returns
/// * 估算的手续费（satoshi）
pub fn estimate_bitcoin_fee(
    input_count: usize,
    output_count: usize,
    fee_per_byte: u64,
) -> u64 {
    // transaction大小估算
    // 基础: 10 bytes
    // 输入: 148 bytes each
    // 输出: 34 bytes each
    let tx_size = 10 + (input_count * 148) + (output_count * 34);
    
    let fee = (tx_size as u64) * fee_per_byte;
    
    debug!("Estimated fee: {} satoshi ({} bytes × {} sat/byte)", 
           fee, tx_size, fee_per_byte);
    
    fee
}

/// fetch推荐的手续费率（satoshi/byte）
///
/// # Returns
/// * 快速: 20 sat/byte
/// * 标准: 10 sat/byte  
/// * 经济: 5 sat/byte
pub async fn get_recommended_fee_rate() -> Result<u64, WalletError> {
    // TODO: fromBlockstream API或mempool.spacefetch实时费率
    // 当前返回标准费率
    
    info!("Using default fee rate: 10 sat/byte");
    Ok(10) // 标准费率
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_select_utxos_simple() {
        let utxos = vec![
            UTXO {
                txid: "abc123".to_string(),
                vout: 0,
                value: 100_000, // 0.001 BTC
                script_pubkey: None,
                confirmations: Some(6),
            },
            UTXO {
                txid: "def456".to_string(),
                vout: 1,
                value: 500_000, // 0.005 BTC
                script_pubkey: None,
                confirmations: Some(6),
            },
        ];
        
        // 目标: 0.003 BTC = 300,000 satoshi
        // 费率: 10 sat/byte
        let result = select_utxos(utxos, 300_000, 10);
        
        assert!(result.is_ok());
        let (selected, change) = result.unwrap();
        
        // 应该选择0.005 BTC的UTXO（最大优先）
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].value, 500_000);
        
        // 找零应该是 500,000 - 300,000 - fee
        assert!(change > 0);
        
        println!("Selected: {} UTXOs, change: {} satoshi", selected.len(), change);
    }
    
    #[test]
    fn test_select_utxos_insufficient() {
        let utxos = vec![
            UTXO {
                txid: "abc123".to_string(),
                vout: 0,
                value: 10_000, // 只有0.0001 BTC
                script_pubkey: None,
                confirmations: Some(6),
            },
        ];
        
        // 目标: 1 BTC = 100,000,000 satoshi（远超balance）
        let result = select_utxos(utxos, 100_000_000, 10);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_estimate_fee() {
        // 1个输入，2个输出（接收+找零）
        let fee = estimate_bitcoin_fee(1, 2, 10);
        
        // 大约 (10 + 148 + 68) = 226 bytes
        // 226 × 10 = 2,260 satoshi
        assert!(fee > 2000 && fee < 3000);
        
        println!("Estimated fee: {} satoshi", fee);
    }
}

