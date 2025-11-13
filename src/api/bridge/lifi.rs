//! LI.FI 跨链桥接 API 集成

use crate::core::errors::WalletError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// LI.FI API 客户端
pub struct LiFiClient {
    client: Client,
    base_url: String,
}

/// LI.FI 路由响应
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LiFiRoutesResponse {
    routes: Vec<LiFiRoute>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct LiFiRoute {
    id: String,
    #[serde(rename = "fromChainId")]
    from_chain_id: u64,
    #[serde(rename = "toChainId")]
    to_chain_id: u64,
    #[serde(rename = "fromToken")]
    from_token: TokenInfo,
    #[serde(rename = "toToken")]
    to_token: TokenInfo,
    #[serde(rename = "fromAmount")]
    from_amount: String,
    #[serde(rename = "toAmount")]
    to_amount: String,
    steps: Vec<Step>,
    #[serde(rename = "gasCosts")]
    gas_costs: Option<Vec<GasCost>>,
}

#[derive(Debug, Deserialize, Clone)]
struct TokenInfo {
    address: String,
    symbol: String,
    decimals: u32,
    #[serde(rename = "chainId")]
    chain_id: u64,
}

#[derive(Debug, Deserialize, Clone)]
struct Step {
    #[serde(rename = "type")]
    step_type: String,
    tool: String,
    action: Action,
}

#[derive(Debug, Deserialize, Clone)]
struct Action {
    #[serde(rename = "fromToken")]
    from_token: TokenInfo,
    #[serde(rename = "toToken")]
    to_token: TokenInfo,
}

#[derive(Debug, Deserialize, Clone)]
struct GasCost {
    #[serde(rename = "type")]
    cost_type: String,
    estimate: String,
    #[serde(rename = "amountUSD")]
    amount_usd: Option<String>,
}

/// 增强的桥接报价
#[derive(Debug, Serialize)]
pub struct EnhancedBridgeQuote {
    pub route_id: String,
    pub from_chain: String,
    pub to_chain: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    pub estimated_time: u32,  // 秒
    pub estimated_fee_usd: f64,
    pub bridges: Vec<String>,
    pub steps: u32,
}

impl LiFiClient {
    /// 创建新的 LI.FI 客户端
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: "https://li.quest/v1".to_string(),
        }
    }

    /// fetch跨链路由
    pub async fn get_routes(
        &self,
        from_chain: &str,
        to_chain: &str,
        from_token: &str,
        to_token: &str,
        amount: &str,
        from_address: &str,
    ) -> Result<Vec<EnhancedBridgeQuote>, WalletError> {
        // fetchChain ID
        let from_chain_id = chain_name_to_id(from_chain)?;
        let to_chain_id = chain_name_to_id(to_chain)?;

        // 构建请求
        let url = format!("{}/advanced/routes", self.base_url);
        
        let body = serde_json::json!({
            "fromChainId": from_chain_id,
            "toChainId": to_chain_id,
            "fromTokenAddress": from_token,
            "toTokenAddress": to_token,
            "fromAmount": amount,
            "fromAddress": from_address,
            "options": {
                "slippage": 0.03,  // 3% 默认滑点
                "order": "RECOMMENDED"
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| WalletError::NetworkError(format!("LI.FI API 请求failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(WalletError::NetworkError(format!(
                "LI.FI API error {}: {}",
                status, body
            )));
        }

        let data: LiFiRoutesResponse = response
            .json()
            .await
            .map_err(|e| WalletError::NetworkError(format!("解析 LI.FI 响应failed: {}", e)))?;

        // 转换为我们的格式
        Ok(data
            .routes
            .into_iter()
            .map(|route| self.convert_route(route, from_chain, to_chain))
            .collect())
    }

    /// 转换路由数据
    fn convert_route(
        &self,
        route: LiFiRoute,
        from_chain: &str,
        to_chain: &str,
    ) -> EnhancedBridgeQuote {
        // 提取桥接工具
        let bridges: Vec<String> = route
            .steps
            .iter()
            .map(|step| step.tool.clone())
            .collect();

        // 计算预估时间（每步30 seconds）
        let estimated_time = route.steps.len() as u32 * 30;

        // 计算总Gas费用
        let estimated_fee_usd = route
            .gas_costs
            .as_ref()
            .and_then(|costs| {
                costs
                    .iter()
                    .filter_map(|cost| cost.amount_usd.as_ref())
                    .filter_map(|s| s.parse::<f64>().ok())
                    .sum::<f64>()
                    .into()
            })
            .unwrap_or(10.0);

        EnhancedBridgeQuote {
            route_id: route.id,
            from_chain: from_chain.to_string(),
            to_chain: to_chain.to_string(),
            from_token: route.from_token.symbol,
            to_token: route.to_token.symbol,
            from_amount: route.from_amount,
            to_amount: route.to_amount,
            estimated_time,
            estimated_fee_usd,
            bridges,
            steps: route.steps.len() as u32,
        }
    }
}

/// 链名称到ID映射
fn chain_name_to_id(name: &str) -> Result<u64, WalletError> {
    match name.to_lowercase().as_str() {
        "eth" | "ethereum" => Ok(1),
        "bsc" | "binance" => Ok(56),
        "polygon" | "matic" => Ok(137),
        "arbitrum" | "arb" => Ok(42161),
        "optimism" | "op" => Ok(10),
        "avalanche" | "avax" => Ok(43114),
        "fantom" | "ftm" => Ok(250),
        _ => Err(WalletError::InvalidInput(format!(
            "不支持的链: {}",
            name
        ))),
    }
}

/// from环境变量check是否启用 LI.FI
pub fn is_lifi_enabled() -> bool {
    std::env::var("ENABLE_LIFI")
        .ok()
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

