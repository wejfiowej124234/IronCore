use anyhow::{anyhow, Result};
use hex;
use reqwest::Client;
use serde_json::json;

use crate::core::domain::Tx;
use crate::security::redaction::redact_body;

/// Return a default RPC node URL (can be replaced by configuration later).
pub fn select_node() -> Option<String> {
    // Keep simple for now — could read env var or config in future.
    Some("https://mainnet.infura.io/v3/".to_string())
}

pub struct NodeManager {
    client: Client,
    rpc_url: String,
}

impl NodeManager {
    /// Create a NodeManager pointing at a given RPC URL.
    pub fn new(rpc_url: &str) -> Self {
        Self { client: Client::new(), rpc_url: rpc_url.to_string() }
    }

    /// Convenience constructor for Infura (requires a project id).
    pub fn new_infura(project_id: &str) -> Self {
        let rpc_url = format!("https://mainnet.infura.io/v3/{}", project_id);
        Self { client: Client::new(), rpc_url }
    }

    /// Send a raw transaction via JSON-RPC eth_sendRawTransaction.
    /// Expects Tx::serialize() to return raw bytes of the signed transaction.
    pub async fn send_tx(&self, tx: Tx) -> Result<String> {
        let raw_bytes = tx.serialize()?;
        let raw_hex = format!("0x{}", hex::encode(raw_bytes));
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [ raw_hex ],
            "id": 1
        });

        let resp =
            self.client.post(&self.rpc_url).json(&payload).send().await.map_err(|e| anyhow!(e))?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.map_err(|e| anyhow!(e))?;

        if !status.is_success() {
            return Err(anyhow!(
                "rpc error status: {} body: {}",
                status,
                redact_body(&body.to_string())
            ));
        }
        if let Some(result) = body.get("result").and_then(|v| v.as_str()) {
            Ok(result.to_string())
        } else if let Some(err) = body.get("error") {
            Err(anyhow!("rpc returned error: {}", redact_body(&err.to_string())))
        } else {
            Err(anyhow!("unexpected rpc response: {}", redact_body(&body.to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_node_and_infura_url() {
        // select_node returns a default base
        let node = select_node();
        assert!(node.is_some());
        // Infura constructor produces expected URL format
        let nm = NodeManager::new_infura("my-project-id");
        assert!(nm.rpc_url.contains("infura.io"));
        assert!(nm.rpc_url.ends_with("my-project-id"));
    }
}
