//! DEX 交换相关数据类型

use serde::{Deserialize, Serialize};

/// 交换报价请求
#[derive(Debug, Clone, Deserialize)]
pub struct SwapQuoteRequest {
    /// 源代币符号（如 "ETH"）
    pub from: String,
    /// 目标代币符号（如 "USDT"）
    pub to: String,
    /// 交换数量
    pub amount: String,
    /// network（如 "eth", "bsc", "polygon"）
    pub network: String,
}

/// 交换路由步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStep {
    /// DEX协议名称（如 "Uniswap V3"）
    pub protocol: String,
    /// 源代币
    pub from: String,
    /// 目标代币
    pub to: String,
    /// 此路径占比（百分比）
    pub percentage: u32,
    /// 池子address（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<String>,
}

/// 交换报价响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    /// 源代币符号
    pub from_token: String,
    /// 目标代币符号
    pub to_token: String,
    /// 源代币数量
    pub from_amount: String,
    /// 预计获得的目标代币数量
    pub to_amount: String,
    /// 交换汇率
    pub exchange_rate: f64,
    /// 价格影响（百分比）
    pub price_impact: f64,
    /// 交换路径
    pub route: Vec<RouteStep>,
    /// Gas 估算（ETH 单位）
    pub gas_estimate: String,
    /// Gas 费用（USD）
    pub estimated_gas_usd: f64,
    /// 报价有效期（秒）
    #[serde(default)]
    pub valid_for: u32,
}

/// 执行交换请求
#[derive(Debug, Clone, Deserialize)]
pub struct SwapExecuteRequest {
    /// Wallet name
    pub wallet_name: String,
    /// 源代币符号
    pub from_token: String,
    /// 目标代币符号
    pub to_token: String,
    /// 交换数量
    pub amount: String,
    /// network
    pub network: String,
    /// 滑点容忍度（百分比，如 0.5 表示 0.5%）
    pub slippage: f64,
    /// walletPassword（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 幂等键
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_request_id: Option<String>,
}

/// 执行交换响应
#[derive(Debug, Clone, Serialize)]
pub struct SwapExecuteResponse {
    /// Transaction hash
    pub tx_id: String,
    /// transaction状态
    pub status: String,
    /// 实际发送的源代币数量
    pub from_amount: String,
    /// 实际获得的目标代币数量
    pub to_amount: String,
    /// 实际成交汇率
    pub actual_rate: f64,
    /// 实际使用的 Gas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<String>,
    /// 区块确认数
    #[serde(default)]
    pub confirmations: u32,
}

/// 代币address映射
pub fn get_token_address(symbol: &str, chain_id: u64) -> Option<String> {
    match (symbol.to_uppercase().as_str(), chain_id) {
        // Ethereum (chain_id: 1)
        ("ETH", 1) => Some("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".to_string()),
        ("USDT", 1) => Some("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string()),
        ("USDC", 1) => Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()),
        ("DAI", 1) => Some("0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string()),
        ("WBTC", 1) => Some("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string()),
        
        // BSC (chain_id: 56)
        ("BNB", 56) => Some("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".to_string()),
        ("USDT", 56) => Some("0x55d398326f99059fF775485246999027B3197955".to_string()),
        ("USDC", 56) => Some("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".to_string()),
        ("BUSD", 56) => Some("0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56".to_string()),
        
        // Polygon (chain_id: 137)
        ("MATIC", 137) => Some("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".to_string()),
        ("USDT", 137) => Some("0xc2132D05D31c914a87C6611C10748AEb04B58e8F".to_string()),
        ("USDC", 137) => Some("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string()),
        
        // Arbitrum (chain_id: 42161)
        ("ETH", 42161) => Some("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".to_string()),
        ("USDT", 42161) => Some("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string()),
        ("USDC", 42161) => Some("0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8".to_string()),
        
        _ => None,
    }
}

/// networkID到Chain ID映射
pub fn network_to_chain_id(network: &str) -> Option<u64> {
    match network.to_lowercase().as_str() {
        "eth" | "ethereum" => Some(1),
        "bsc" | "binance" => Some(56),
        "polygon" | "matic" => Some(137),
        "arbitrum" | "arb" => Some(42161),
        "optimism" | "op" => Some(10),
        "avalanche" | "avax" => Some(43114),
        _ => None,
    }
}

