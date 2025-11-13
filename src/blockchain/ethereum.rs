// ...existing code...
use anyhow::Result;
use async_trait::async_trait;
use ethers::{
    prelude::{JsonRpcClient, *},
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Eip1559TransactionRequest, NameOrAddress, U256},
    utils::parse_ether,
};
use std::{str::FromStr, time::Duration};
use tracing::{debug, info, warn};

use super::traits::{BlockchainClient, TransactionStatus};
use crate::core::errors::WalletError;

#[derive(Clone)]
pub struct EthereumClient<P: JsonRpcClient + Clone = Http> {
    provider: Provider<P>,
    network_name: String,
    chain_id: u64,
}

impl EthereumClient<Http> {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        // Clean RPC URL
        let rpc_url_clean = rpc_url.trim();
        let parsed_url = reqwest::Url::parse(rpc_url_clean).map_err(|e| {
            anyhow::anyhow!(
                "Invalid Ethereum RPC URL '{}': {}. Please check config.toml or env vars.",
                rpc_url_clean,
                e
            )
        })?;

        info!("Connecting to Ethereum network: {}", parsed_url);
        // Build a reqwest client with a short timeout; allow proxy environment vars.
        let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(10));
        if let Ok(proxy) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("HTTP_PROXY")) {
            if let Ok(p) = reqwest::Proxy::all(proxy) {
                builder = builder.proxy(p);
            }
        }
        let client =
            builder.build().map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let provider = Provider::new(Http::new_with_client(parsed_url.clone(), client));

        let chain_id = provider
            .get_chainid()
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to get chain ID from {}. Error: {}. This might be due to a network issue, firewall, or an invalid RPC URL.", parsed_url, e)
            })?
            .as_u64();

        let network_name = match chain_id {
            1 => "ethereum".to_string(),
            11155111 => "sepolia".to_string(),
            137 => "polygon".to_string(),
            56 => "bsc".to_string(),
            97 => "bsctestnet".to_string(),
            _ => format!("ethereum-{}", chain_id),
        };

        info!("Connected to {} (Chain ID: {})", network_name, chain_id);

        Ok(Self { provider, network_name, chain_id })
    }

    pub async fn new_with_chain_id(rpc_url: &str, chain_id: u64) -> Result<Self> {
        info!("Connecting to Ethereum network: {} (Chain ID: {})", rpc_url, chain_id);

        // Reuse new() to build provider/client then override chain_id & network name.
        let temp_client = Self::new(rpc_url).await?;
        let provider = temp_client.provider;

        let network_name = match chain_id {
            1 => "ethereum".to_string(),
            11155111 => "sepolia".to_string(),
            137 => "polygon".to_string(),
            56 => "bsc".to_string(),
            97 => "bsctestnet".to_string(),
            _ => format!("ethereum-{}", chain_id),
        };

        info!("Connected to {} (Chain ID: {})", network_name, chain_id);

        Ok(Self { provider, network_name, chain_id })
    }
}

impl<P: JsonRpcClient + Clone> EthereumClient<P>
where
    // The `ethers` `Provider` requires its client `P` to be `Send + Sync` for async operations.
    // This bound is necessary for the `BlockchainClient` trait methods to be callable.
    P: Send + Sync,
{
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Creates a new EthereumClient with a given provider.
    /// This is useful for testing with a `MockProvider`.
    pub fn new_with_provider(provider: Provider<P>) -> EthereumClient<P> {
        EthereumClient {
            provider,
            network_name: "test".to_string(), // Default network name for testing
            chain_id: 1,                      // Default chain ID for testing (Ethereum Mainnet)
        }
    }

    fn create_wallet_from_private_key(&self, private_key: &[u8]) -> Result<LocalWallet> {
        // Validate length and create a wallet. Do NOT log key material.
        tracing::debug!("create_wallet_from_private_key: incoming len = {}", private_key.len());
        if private_key.len() != 32 {
            return Err(anyhow::anyhow!("Private key must be 32 bytes"));
        }

        let wallet = LocalWallet::from_bytes(private_key)
            .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?
            .with_chain_id(self.chain_id);

        Ok(wallet)
    }

    pub async fn get_gas_price(&self) -> Result<U256> {
        debug!("get_gas_price called");
        let res = self.provider.get_gas_price().await;
        match res {
            Ok(v) => {
                debug!("get_gas_price got = 0x{:x}", v);
                Ok(v)
            }
            Err(e) => Err(anyhow::anyhow!("Failed to get gas price: {}", e)),
        }
    }

    pub async fn get_nonce(&self, address: &Address) -> Result<U256> {
        debug!(address = %hex::encode(address), "get_nonce called for address");
        let res = self.provider.get_transaction_count(*address, None).await;
        match res {
            Ok(v) => {
                debug!("get_nonce got = 0x{:x}", v);
                Ok(v)
            }
            Err(e) => Err(anyhow::anyhow!("Failed to get nonce: {}", e)),
        }
    }
}

#[async_trait]
impl<P> BlockchainClient for EthereumClient<P>
where
    P: JsonRpcClient + Clone + 'static + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn BlockchainClient> {
        Box::new(self.clone())
    }

    async fn get_balance(&self, address: &str) -> Result<String, WalletError> {
        debug!("Getting ETH balance for address: {}", address);

        let address = Address::from_str(address)
            .map_err(|e| WalletError::AddressError(format!("Invalid Ethereum address: {}", e)))?;

        let balance =
            self.provider.get_balance(address, None).await.map_err(|e| {
                WalletError::BlockchainError(format!("Failed to get balance: {}", e))
            })?;

        let balance_eth = ethers::utils::format_ether(balance);
        debug!("Balance: {} ETH", balance_eth);

        Ok(balance_eth)
    }

    async fn send_transaction(
        &self,
        private_key: &crate::core::domain::PrivateKey,
        to: &str,
        amount: &str,
    ) -> Result<String, WalletError> {
        info!("Sending {} ETH to {}", amount, to);

        // Create wallet from private key using scoped secret access
        let wallet = private_key
            .with_secret(|pk_bytes| self.create_wallet_from_private_key(pk_bytes))
            .map_err(|e| {
                WalletError::KeyDerivationError(format!(
                    "Failed to create wallet from private key: {}",
                    e
                ))
            })?;

        // Parse addresses and amount
        let to_address = Address::from_str(to)
            .map_err(|e| WalletError::AddressError(format!("Invalid recipient address: {}", e)))?;

        let amount_wei = parse_ether(amount)
            .map_err(|e| WalletError::ValidationError(format!("Invalid amount: {}", e)))?;

        // Get current gas price and optionally use provided nonce
        let gas_price = self.get_gas_price().await?;
        let nonce = self.get_nonce(&wallet.address()).await?;
        debug!("send_transaction: gas_price = 0x{:x}", gas_price);
        debug!("send_transaction: nonce = 0x{:x}", nonce);

        // Create EIP-1559 transaction (type-2). Derive simple fee settings from gas_price as fallback.
        // Note: This enforces proper chain_id signing via LocalWallet::with_chain_id in create_wallet_from_private_key.
        let max_fee_per_gas = gas_price.saturating_mul(U256::from(2u64));
        let max_priority_fee_per_gas =
            (gas_price / U256::from(10u64)).max(U256::from(1_000_000_000u64)); // >= 1 gwei

        let tx = Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(to_address)),
            value: Some(amount_wei),
            gas: Some(U256::from(21000u64)),
            nonce: Some(nonce),
            max_fee_per_gas: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            ..Default::default()
        };

        // Sign and send transaction
        let client = SignerMiddleware::new(self.provider.clone(), wallet);

        let pending_tx = client.send_transaction(tx, None).await.map_err(|e| {
            WalletError::BlockchainError(format!("Failed to send transaction: {}", e))
        })?;

        // Convert H256 hash to a canonical 0x-prefixed hex string for logs/returns.
        let tx_hash = format!("0x{}", hex::encode(pending_tx.tx_hash().as_bytes()));

        info!(tx_hash = %tx_hash, "Transaction sent");
        Ok(tx_hash)
    }

    /// Allow callers to provide an explicit nonce. If None is provided, fall back
    /// to the default behavior (querying the chain for a nonce).
    async fn send_transaction_with_nonce(
        &self,
        private_key: &crate::core::domain::PrivateKey,
        to: &str,
        amount: &str,
        nonce: Option<u64>,
    ) -> Result<String, WalletError> {
        info!("Sending {} ETH to {} (nonce override: {:?})", amount, to, nonce);

        // Create wallet from private key using scoped secret access
        let wallet = private_key
            .with_secret(|pk_bytes| self.create_wallet_from_private_key(pk_bytes))
            .map_err(|e| {
                WalletError::KeyDerivationError(format!(
                    "Failed to create wallet from private key: {}",
                    e
                ))
            })?;

        // Parse addresses and amount
        let to_address = Address::from_str(to)
            .map_err(|e| WalletError::AddressError(format!("Invalid recipient address: {}", e)))?;

        let amount_wei = parse_ether(amount)
            .map_err(|e| WalletError::ValidationError(format!("Invalid amount: {}", e)))?;

        // Get current gas price and optionally use provided nonce
        let gas_price = self.get_gas_price().await?;
        let nonce_u256 = if let Some(n) = nonce {
            U256::from(n)
        } else {
            self.get_nonce(&wallet.address()).await?
        };

        debug!("send_transaction: gas_price = 0x{:x}", gas_price);
        debug!("send_transaction: nonce = 0x{:x}", nonce_u256);

        let max_fee_per_gas = gas_price.saturating_mul(U256::from(2u64));
        let max_priority_fee_per_gas =
            (gas_price / U256::from(10u64)).max(U256::from(1_000_000_000u64)); // >= 1 gwei

        let tx = Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(to_address)),
            value: Some(amount_wei),
            gas: Some(U256::from(21000u64)),
            nonce: Some(nonce_u256),
            max_fee_per_gas: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            ..Default::default()
        };

        // Sign and send transaction
        let client = SignerMiddleware::new(self.provider.clone(), wallet);

        let pending_tx = client.send_transaction(tx, None).await.map_err(|e| {
            WalletError::BlockchainError(format!("Failed to send transaction: {}", e))
        })?;

        let tx_hash = format!("0x{}", hex::encode(pending_tx.tx_hash().as_bytes()));

        info!(tx_hash = %tx_hash, "Transaction sent");
        Ok(tx_hash)
    }

    async fn get_transaction_status(
        &self,
        tx_hash: &str,
    ) -> Result<TransactionStatus, WalletError> {
        debug!("Getting transaction status for: {}", tx_hash);

        let tx_hash = H256::from_str(tx_hash).map_err(|e| {
            WalletError::ValidationError(format!("Invalid transaction hash: {}", e))
        })?;

        match self.provider.get_transaction_receipt(tx_hash).await {
            Ok(Some(receipt)) => {
                let status = if receipt.status == Some(U64::from(1)) {
                    TransactionStatus::Confirmed
                } else {
                    TransactionStatus::Failed
                };
                debug!("Transaction status: {:?}", status);
                Ok(status)
            }
            Ok(None) => {
                // Transaction exists but not mined yet
                match self.provider.get_transaction(tx_hash).await {
                    Ok(Some(_)) => Ok(TransactionStatus::Pending),
                    Ok(None) => {
                        // If both receipt and transaction are not found, the transaction is unknown.
                        Ok(TransactionStatus::Unknown)
                    }
                    Err(e) => Err(WalletError::BlockchainError(format!(
                        "Failed to get transaction details for {}: {}",
                        tx_hash, e
                    ))),
                }
            }
            Err(e) => {
                warn!("Failed to get transaction receipt for {}: {}", tx_hash, e);
                Err(WalletError::BlockchainError(format!(
                    "Failed to get transaction receipt: {}",
                    e
                )))
            }
        }
    }

    async fn estimate_fee(&self, to_address: &str, amount: &str) -> Result<String, WalletError> {
        debug!("Estimating fee for {} ETH to {}", amount, to_address);

        let _to_address = Address::from_str(to_address)
            .map_err(|e| WalletError::AddressError(format!("Invalid recipient address: {}", e)))?;

        let _amount_wei = parse_ether(amount)
            .map_err(|e| WalletError::ValidationError(format!("Invalid amount: {}", e)))?;

        let gas_price =
            self.get_gas_price().await.map_err(|e| WalletError::BlockchainError(e.to_string()))?;
        let gas_limit = U256::from(21000u64); // Standard ETH transfer

        let total_fee = gas_price * gas_limit;
        let fee_eth = ethers::utils::format_ether(total_fee);

        debug!("Estimated fee: {} ETH", fee_eth);
        Ok(fee_eth)
    }

    async fn get_block_number(&self) -> Result<u64, WalletError> {
        let block_number = self.provider.get_block_number().await.map_err(|e| {
            WalletError::BlockchainError(format!("Failed to get block number: {}", e))
        })?;

        Ok(block_number.as_u64())
    }

    async fn get_nonce(&self, address: &str) -> Result<u64, WalletError> {
        debug!("Getting nonce for address: {}", address);

        let address = Address::from_str(address)
            .map_err(|e| WalletError::AddressError(format!("Invalid Ethereum address: {}", e)))?;

        let nonce = self
            .provider
            .get_transaction_count(address, None)
            .await
            .map_err(|e| WalletError::BlockchainError(format!("Failed to get nonce: {}", e)))?;

        debug!("Current nonce: {}", nonce);
        Ok(nonce.as_u64())
    }

    fn validate_address(&self, address: &str) -> anyhow::Result<bool> {
        match Address::from_str(address) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn get_network_name(&self) -> &str {
        &self.network_name
    }

    fn get_native_token(&self) -> &str {
        "ETH"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Http, Provider};
    use std::convert::TryFrom;

    // helper to build a client without requiring a live RPC
    fn make_local_client() -> EthereumClient<Http> {
        let provider =
            Provider::<Http>::try_from("http://127.0.0.1:8545").expect("provider url ok");
        EthereumClient::new_with_provider(provider)
    }

    #[test]
    fn create_wallet_from_private_key_success() {
        let _client = make_local_client();
        // initialize the 32-byte test key at runtime to avoid source-level secret literal
        let mut key = [0u8; 32];
        for b in key.iter_mut() {
            *b = 0x11u8;
        }
        // Use the helper call to validate that wallet creation works; call via helper directly
        let wallet =
            EthereumClient::new_with_provider(Provider::try_from("http://127.0.0.1:8545").unwrap())
                .create_wallet_from_private_key(&key)
                .expect("should create wallet");
        let _addr = wallet.address(); // basic smoke check
    }

    #[test]
    fn create_wallet_from_private_key_invalid_length() {
        let client = make_local_client();
        let short_key = [0u8; 16];
        let res = client.create_wallet_from_private_key(&short_key);
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("32") || msg.contains("Private key"), "unexpected err: {}", msg);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn send_transaction_short_key_fails_fast() {
        let _client = make_local_client();
        let short_key = [0u8; 16];
        // Construction of PrivateKey should fail for short keys
        let try_pk = crate::core::domain::PrivateKey::try_from_slice(&short_key);
        assert!(try_pk.is_err());
    }

    #[test]
    fn test_address_validation_smoke() {
        let client = make_local_client();
        assert!(client.validate_address("0x742d35Cc6634C0532925a3b8D400e8B78fFe4860").unwrap());
        assert!(!client.validate_address("not-an-address").unwrap());
    }
}
// ...existing code...
