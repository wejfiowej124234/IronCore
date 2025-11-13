// filepath: src/blockchain/bridge/mock.rs
use crate::blockchain::bridge::relay::{mock_bridge_transfer, mock_check_transfer_status};
use crate::blockchain::bridge::BridgeTransactionStatus;
use crate::blockchain::traits::Bridge;
use crate::core::wallet_info::SecureWalletData;
use anyhow::Result;
use async_trait::async_trait;

/// Ethereum -> BSC mock bridge.
#[derive(Debug, Clone)]
pub struct EthereumToBSCBridge {
    pub contract_address: String,
}

impl EthereumToBSCBridge {
    pub fn new(addr: &str) -> Self {
        Self { contract_address: addr.to_string() }
    }
}

#[async_trait]
impl Bridge for EthereumToBSCBridge {
    async fn transfer_across_chains(
        &self,
        from_chain: &str,
        to_chain: &str,
        token: &str,
        amount: &str,
        wallet_data: &SecureWalletData,
    ) -> Result<String> {
        mock_bridge_transfer(
            from_chain,
            to_chain,
            token,
            amount,
            &self.contract_address,
            wallet_data,
        )
        .await
    }

    async fn check_transfer_status(&self, tx_id: &str) -> Result<BridgeTransactionStatus> {
        mock_check_transfer_status(tx_id).await
    }
}

/// Polygon -> Ethereum mock bridge.
#[derive(Debug, Clone)]
pub struct PolygonToEthereumBridge {
    pub contract_address: String,
}

impl PolygonToEthereumBridge {
    pub fn new(addr: &str) -> Self {
        Self { contract_address: addr.to_string() }
    }
}

#[async_trait]
impl Bridge for PolygonToEthereumBridge {
    async fn transfer_across_chains(
        &self,
        from_chain: &str,
        to_chain: &str,
        token: &str,
        amount: &str,
        wallet_data: &SecureWalletData,
    ) -> Result<String> {
        mock_bridge_transfer(
            from_chain,
            to_chain,
            token,
            amount,
            &self.contract_address,
            wallet_data,
        )
        .await
    }

    async fn check_transfer_status(&self, tx_id: &str) -> Result<BridgeTransactionStatus> {
        mock_check_transfer_status(tx_id).await
    }
}
