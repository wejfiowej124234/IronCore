//! transaction历史适配器
//! 
//! 将后端的 TransactionInfo 适配为前端期望的格式

use serde::Serialize;

/// 前端期望的transaction历史响应
#[derive(Debug, Serialize)]
pub struct FrontendTransactionHistoryResponse {
    pub transactions: Vec<FrontendTransaction>,
}

/// 前端期望的transaction记录格式
#[derive(Debug, Clone, Serialize)]
pub struct FrontendTransaction {
    /// transactionID（映射自hash）
    pub id: String,
    /// 时间戳（Unix秒）
    pub timestamp: u64,
    /// Sender address（映射自from）
    pub from_address: String,
    /// Recipient address（映射自to）
    pub to_address: String,
    /// 金额（映射自amount或value）
    pub amount: String,
    /// 状态
    pub status: String,
    /// network
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// 手续费
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<String>,
    /// 确认数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<u32>,
    /// Transaction hash（与id相同）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    /// transaction类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// 代币符号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokenSymbol: Option<String>,
}

/// 将后端TransactionInfo转换为前端期望的格式
pub fn adapt_transaction(
    tx: &crate::blockchain::traits::Transaction,
    network: Option<&str>,
) -> FrontendTransaction {
    FrontendTransaction {
        id: tx.hash.clone(),
        timestamp: 0, // 需要fromblockchainfetch
        from_address: tx.from.clone(),
        to_address: tx.to.clone(),
        amount: tx.amount.clone(),
        status: "confirmed".to_string(), // 默认已确认
        network: network.map(|s| s.to_string()),
        fee: None,
        confirmations: Some(6), // 默认6个确认
        tx_hash: Some(tx.hash.clone()),
        r#type: Some("transfer".to_string()),
        tokenSymbol: None,
    }
}

/// 适配transaction历史响应
pub fn adapt_transaction_history(
    transactions: Vec<crate::blockchain::traits::Transaction>,
    network: Option<&str>,
) -> Vec<FrontendTransaction> {
    transactions
        .into_iter()
        .map(|tx| adapt_transaction(&tx, network))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapt_transaction() {
        let tx = crate::blockchain::traits::Transaction {
            hash: "0xabc123".to_string(),
            from: "0x111".to_string(),
            to: "0x222".to_string(),
            amount: "1.5".to_string(),
        };

        let adapted = adapt_transaction(&tx, Some("eth"));

        assert_eq!(adapted.id, "0xabc123");
        assert_eq!(adapted.from_address, "0x111");
        assert_eq!(adapted.to_address, "0x222");
        assert_eq!(adapted.amount, "1.5");
        assert_eq!(adapted.tx_hash, Some("0xabc123".to_string()));
    }
}

