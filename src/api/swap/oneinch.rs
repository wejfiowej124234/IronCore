//! 1inch Aggregation API 集成

use super::types::*;
use crate::core::errors::WalletError;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// 1inch API 客户端
pub struct OneInchClient {
    client: Client,
    api_key: Option<String>,
    base_url: String,
}

/// 1inch API 报价响应（简化版）
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OneInchQuoteResponse {
    #[serde(rename = "toAmount")]
    to_amount: String,
    #[serde(rename = "fromToken")]
    from_token: TokenInfo,
    #[serde(rename = "toToken")]
    to_token: TokenInfo,
    #[serde(default)]
    protocols: Vec<Vec<Vec<ProtocolRoute>>>,
    #[serde(rename = "estimatedGas")]
    estimated_gas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenInfo {
    symbol: String,
    address: String,
    decimals: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ProtocolRoute {
    name: String,
    #[serde(rename = "fromTokenAddress")]
    from_token_address: String,
    #[serde(rename = "toTokenAddress")]
    to_token_address: String,
    part: f64,
}

/// 1inch Swap 响应
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OneInchSwapResponse {
    tx: TransactionData,
    #[serde(rename = "toAmount")]
    to_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionData {
    pub from: String,
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas: Option<String>,
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<String>,
}

impl OneInchClient {
    /// 创建新的 1inch 客户端
    pub fn new(api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            api_key,
            base_url: "https://api.1inch.dev".to_string(),
        }
    }

    /// fetch交换报价
    pub async fn get_quote(
        &self,
        chain_id: u64,
        from_token: &str,
        to_token: &str,
        amount: &str,
    ) -> Result<SwapQuote, WalletError> {
        // 构建URL
        let url = format!(
            "{}/swap/v5.2/{}/quote",
            self.base_url, chain_id
        );

        // 构建请求
        let mut request = self
            .client
            .get(&url)
            .query(&[
                ("src", from_token),
                ("dst", to_token),
                ("amount", amount),
            ]);

        // 添加 API Key（如果有）
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        // 发送请求
        let response = request
            .send()
            .await
            .map_err(|e| WalletError::NetworkError(format!("1inch API 请求failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(WalletError::NetworkError(format!(
                "1inch API 返回error {}: {}",
                status, body
            )));
        }

        let data: OneInchQuoteResponse = response
            .json()
            .await
            .map_err(|e| WalletError::NetworkError(format!("解析 1inch 响应failed: {}", e)))?;

        // 转换为我们的格式
        self.convert_quote_response(data, from_token, to_token, amount)
    }

    /// 执行交换（fetchtransaction数据）
    pub async fn get_swap_tx(
        &self,
        chain_id: u64,
        from_token: &str,
        to_token: &str,
        amount: &str,
        from_address: &str,
        slippage: f64,
    ) -> Result<TransactionData, WalletError> {
        let url = format!("{}/swap/v5.2/{}/swap", self.base_url, chain_id);

        let mut request = self
            .client
            .get(&url)
            .query(&[
                ("src", from_token),
                ("dst", to_token),
                ("amount", amount),
                ("from", from_address),
                ("slippage", &slippage.to_string()),
                ("disableEstimate", "true"),
            ]);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| WalletError::NetworkError(format!("1inch swap API failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(WalletError::NetworkError(format!(
                "1inch swap API error {}: {}",
                status, body
            )));
        }

        let data: OneInchSwapResponse = response
            .json()
            .await
            .map_err(|e| WalletError::NetworkError(format!("解析 swap 响应failed: {}", e)))?;

        Ok(data.tx)
    }

    /// 转换 1inch 响应为我们的格式
    fn convert_quote_response(
        &self,
        data: OneInchQuoteResponse,
        from_symbol: &str,
        to_symbol: &str,
        from_amount: &str,
    ) -> Result<SwapQuote, WalletError> {
        // 解析数量
        let to_amount = data.to_amount.clone();
        let from_amount_f64 = from_amount.parse::<f64>()
            .map_err(|_| WalletError::InvalidInput("无效的源代币数量".to_string()))?;
        let to_amount_f64 = to_amount.parse::<f64>()
            .map_err(|_| WalletError::InvalidInput("无效的目标代币数量".to_string()))?;

        // 计算汇率
        let exchange_rate = if from_amount_f64 > 0.0 {
            to_amount_f64 / from_amount_f64
        } else {
            0.0
        };

        // 提取路由信息
        let mut route = Vec::new();
        for protocol_group in &data.protocols {
            for route_group in protocol_group {
                for protocol_route in route_group {
                    route.push(RouteStep {
                        protocol: protocol_route.name.clone(),
                        from: from_symbol.to_string(),
                        to: to_symbol.to_string(),
                        percentage: (protocol_route.part * 100.0) as u32,
                        pool: None,
                    });
                }
            }
        }

        // 如果没有路由信息，添加一个默认的
        if route.is_empty() {
            route.push(RouteStep {
                protocol: "1inch Aggregator".to_string(),
                from: from_symbol.to_string(),
                to: to_symbol.to_string(),
                percentage: 100,
                pool: None,
            });
        }

        // Gas 估算
        let gas_estimate = data
            .estimated_gas
            .unwrap_or_else(|| "150000".to_string());
        
        // 估算 Gas 费用（USD）- 简化计算
        let gas_f64 = gas_estimate.parse::<f64>().unwrap_or(150000.0);
        let estimated_gas_usd = (gas_f64 * 0.00000003 * 1900.0).max(0.1); // 粗略估算

        // 价格影响（简化：根据数量估算）
        let price_impact = if from_amount_f64 > 10.0 {
            0.5
        } else if from_amount_f64 > 1.0 {
            0.1
        } else {
            0.05
        };

        Ok(SwapQuote {
            from_token: from_symbol.to_string(),
            to_token: to_symbol.to_string(),
            from_amount: from_amount.to_string(),
            to_amount,
            exchange_rate,
            price_impact,
            route,
            gas_estimate,
            estimated_gas_usd,
            valid_for: 30, // 30 seconds有效期
        })
    }
}

/// from环境变量fetch API Key
pub fn get_oneinch_api_key() -> Option<String> {
    std::env::var("ONEINCH_API_KEY").ok()
}

