use anyhow::Result;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

/// Transaction types supported by the wallet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    /// Native token transfer (ETH, etc.)
    NativeTransfer,
    /// ERC-20 token transfer
    Erc20Transfer { token_address: String, token_amount: String },
    /// Smart contract call
    ContractCall { contract_address: String, data: Vec<u8>, value: Option<String> },
}

/// Complete transaction structure with all necessary fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    /// Transaction type
    pub tx_type: TransactionType,
    /// Recipient address
    pub to: String,
    /// Amount for native transfers (in wei for ETH)
    pub amount: String,
    pub network: String,
    /// Optional gas price (for Ethereum-like chains)
    pub gas_price: Option<String>,
    /// Optional gas limit (for Ethereum-like chains)
    pub gas_limit: Option<String>,
    /// Optional data payload (for contract calls)
    pub data: Option<Vec<u8>>,
    /// Optional nonce (for replay protection)
    pub nonce: Option<u64>,
    /// Optional chain ID (for multi-chain support)
    pub chain_id: Option<u64>,
    /// Transaction timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Tx {
    /// Encode an ERC-20 transfer calldata from recipient and minimal-unit amount using canonical ABI helpers.
    /// selector: keccak256("transfer(address,uint256)")[0..4] = a9059cbb
    /// data: 4 + 32 (address padded) + 32 (amount padded)
    pub fn encode_erc20_transfer_calldata(
        to_address_hex: &str,
        amount_min_units: &str,
    ) -> Result<Vec<u8>> {
        let selector = crate::core::abi::selector_from_signature("transfer(address,uint256)");
        let addr = crate::core::abi::abi_word_address(to_address_hex)
            .map_err(|_| anyhow::anyhow!("Invalid ERC-20 address format for calldata"))?;
        let amt = crate::core::abi::abi_word_uint256_from_str(amount_min_units)?;
        Ok(crate::core::abi::abi_pack(selector, &[addr, amt]))
    }

    /// Encode a generic static ABI contract call from selector and 32-byte words.
    pub fn encode_contract_call_static(selector: [u8; 4], words: &[[u8; 32]]) -> Vec<u8> {
        crate::core::abi::abi_pack(selector, words)
    }
    /// Parse a string containing only digits into u128.
    fn parse_u128_str_digits(s: &str) -> Result<u128> {
        if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow::anyhow!("Amount must be an integer string in minimal units"));
        }
        s.parse::<u128>().map_err(|_| anyhow::anyhow!("Amount exceeds supported range"))
    }

    /// Normalize native amount to minimal units (wei/lamports) as u128; requires integer input.
    fn normalized_native_amount(&self) -> Result<u128> {
        // We require inputs already expressed in minimal units for safety and determinism.
        // This avoids decimal parsing ambiguity and rounding.
        Self::parse_u128_str_digits(&self.amount)
    }

    /// Minimal big-endian bytes for an unsigned integer (no leading zeros; zero -> [0]).
    fn be_bytes_min_u128(x: u128) -> Vec<u8> {
        let buf = x.to_be_bytes();
        // strip leading zeros but leave at least one byte (for zero)
        let first_non_zero = buf.iter().position(|&b| b != 0).unwrap_or(buf.len() - 1);
        buf[first_non_zero..].to_vec()
    }

    /// Create a new native token transfer transaction
    pub fn new_native_transfer(to: &str, amount: &str, network: &str) -> Self {
        Self {
            tx_type: TransactionType::NativeTransfer,
            to: to.to_string(),
            amount: amount.to_string(),
            network: network.to_string(),
            gas_price: None,
            gas_limit: None,
            data: None,
            nonce: None,
            chain_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new ERC-20 token transfer transaction
    pub fn new_erc20_transfer(
        token_address: &str,
        to: &str,
        token_amount: &str,
        network: &str,
    ) -> Self {
        Self {
            tx_type: TransactionType::Erc20Transfer {
                token_address: token_address.to_string(),
                token_amount: token_amount.to_string(),
            },
            to: to.to_string(),
            amount: "0".to_string(), // ERC-20 transfers have 0 ETH value
            network: network.to_string(),
            gas_price: None,
            gas_limit: None,
            data: None,
            nonce: None,
            chain_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new contract call transaction
    pub fn new_contract_call(
        contract_address: &str,
        data: Vec<u8>,
        value: Option<&str>,
        network: &str,
    ) -> Self {
        Self {
            tx_type: TransactionType::ContractCall {
                contract_address: contract_address.to_string(),
                data: data.clone(),
                value: value.map(|s| s.to_string()),
            },
            to: contract_address.to_string(),
            amount: value.unwrap_or("0").to_string(),
            network: network.to_string(),
            gas_price: None,
            gas_limit: None,
            data: Some(data),
            nonce: None,
            chain_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Set gas parameters for Ethereum-like transactions
    pub fn with_gas(mut self, gas_price: &str, gas_limit: &str) -> Self {
        self.gas_price = Some(gas_price.to_string());
        self.gas_limit = Some(gas_limit.to_string());
        self
    }

    /// Set nonce for replay protection
    pub fn with_nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Set chain ID for multi-chain support
    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Serialize transaction to JSON bytes with input validation
    pub fn serialize(&self) -> Result<Vec<u8>> {
        // Validate all fields before serialization to prevent injection attacks
        self.validate_serialization_fields()?;

        // Use serde_json for serialization (safe after validation)
        serde_json::to_vec(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize transaction: {}", e))
    }

    /// Validate fields for safe serialization (prevent injection attacks)
    fn validate_serialization_fields(&self) -> Result<()> {
        // Validate recipient address
        if self.to.len() > 100 {
            return Err(anyhow::anyhow!("Recipient address too long"));
        }
        if !self.to.chars().all(|c| c.is_alphanumeric() || c == 'x' || c == 'X' || c == '_') {
            return Err(anyhow::anyhow!("Invalid characters in recipient address"));
        }

        // Validate amount (must be integer string in minimal units for supported networks)
        if self.amount.len() > 50 {
            return Err(anyhow::anyhow!("Amount string too long"));
        }
        if !self.amount.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow::anyhow!("Invalid amount: must be integer minimal units"));
        }

        // Validate network name
        if self.network.len() > 20 {
            return Err(anyhow::anyhow!("Network name too long"));
        }
        if !self.network.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(anyhow::anyhow!("Invalid characters in network name"));
        }

        // Validate gas parameters if present
        if let Some(gas_price) = &self.gas_price {
            if gas_price.len() > 30 {
                return Err(anyhow::anyhow!("Gas price string too long"));
            }
            if !gas_price.chars().all(|c| c.is_numeric()) {
                return Err(anyhow::anyhow!("Invalid characters in gas price"));
            }
        }

        if let Some(gas_limit) = &self.gas_limit {
            if gas_limit.len() > 30 {
                return Err(anyhow::anyhow!("Gas limit string too long"));
            }
            if !gas_limit.chars().all(|c| c.is_numeric()) {
                return Err(anyhow::anyhow!("Invalid characters in gas limit"));
            }
        }

        // Validate data payload size
        if let Some(data) = &self.data {
            if data.len() > 10000 {
                // 10KB limit
                return Err(anyhow::anyhow!("Transaction data payload too large"));
            }
        }

        // Validate transaction type specific fields
        match &self.tx_type {
            TransactionType::Erc20Transfer { token_address, token_amount } => {
                if token_address.len() > 100 {
                    return Err(anyhow::anyhow!("Token address too long"));
                }
                if !token_address
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == 'x' || c == 'X' || c == '_')
                {
                    return Err(anyhow::anyhow!("Invalid characters in token address"));
                }
                if token_amount.len() > 50 {
                    return Err(anyhow::anyhow!("Token amount string too long"));
                }
                if !token_amount.chars().all(|c| c.is_ascii_digit()) {
                    return Err(anyhow::anyhow!(
                        "Invalid token amount: must be integer minimal units"
                    ));
                }
            }
            TransactionType::ContractCall { contract_address, data: call_data, value } => {
                if contract_address.len() > 100 {
                    return Err(anyhow::anyhow!("Contract address too long"));
                }
                if !contract_address
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == 'x' || c == 'X' || c == '_')
                {
                    return Err(anyhow::anyhow!("Invalid characters in contract address"));
                }
                if let Some(val) = value {
                    if val.len() > 50 {
                        return Err(anyhow::anyhow!("Contract call value string too long"));
                    }
                    if !val.chars().all(|c| c.is_ascii_digit()) {
                        return Err(anyhow::anyhow!(
                            "Invalid contract call value: must be integer minimal units"
                        ));
                    }
                }
                if call_data.len() > 10000 {
                    // 10KB limit
                    return Err(anyhow::anyhow!("Contract call data payload too large"));
                }
            }
            TransactionType::NativeTransfer => {
                // Already validated above
            }
        }

        Ok(())
    }

    /// Deserialize transaction from JSON bytes with validation
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        // Limit input size to prevent DoS attacks
        if data.len() > 50000 {
            // 50KB limit
            return Err(anyhow::anyhow!("Transaction data too large"));
        }

        let tx: Self = serde_json::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize transaction: {}", e))?;

        // Validate deserialized data
        tx.validate_serialization_fields()?;

        Ok(tx)
    }

    /// Validate transaction fields based on type and network
    pub fn validate(&self) -> Result<()> {
        // Validate basic fields
        if self.to.is_empty() {
            return Err(anyhow::anyhow!("Recipient address cannot be empty"));
        }

        if self.amount.is_empty() {
            return Err(anyhow::anyhow!("Amount cannot be empty"));
        }

        if self.network.is_empty() {
            return Err(anyhow::anyhow!("Network cannot be empty"));
        }

        // Validate network-specific requirements
        match self.network.as_str() {
            "eth" | "sepolia" | "polygon" | "bsc" => {
                self.validate_ethereum_transaction()?;
            }
            _ => return Err(anyhow::anyhow!("Unsupported network: {}", self.network)),
        }

        // Validate transaction type specific fields
        match &self.tx_type {
            TransactionType::NativeTransfer => {
                // Basic validation already done
            }
            TransactionType::Erc20Transfer { token_address, token_amount } => {
                if token_address.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Token address cannot be empty for ERC-20 transfer"
                    ));
                }
                if token_amount.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Token amount cannot be empty for ERC-20 transfer"
                    ));
                }
            }
            TransactionType::ContractCall { contract_address, data, .. } => {
                if contract_address.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Contract address cannot be empty for contract call"
                    ));
                }
                if data.is_empty() {
                    return Err(anyhow::anyhow!("Contract call data cannot be empty"));
                }
            }
        }

        Ok(())
    }

    /// Validate Ethereum-specific transaction fields
    fn validate_ethereum_transaction(&self) -> Result<()> {
        // For Ethereum transactions, gas parameters are required
        if self.gas_price.is_none() {
            return Err(anyhow::anyhow!("Gas price is required for Ethereum transactions"));
        }
        if self.gas_limit.is_none() {
            return Err(anyhow::anyhow!("Gas limit is required for Ethereum transactions"));
        }
        if self.nonce.is_none() {
            return Err(anyhow::anyhow!("Nonce is required for Ethereum transactions"));
        }
        if self.chain_id.is_none() {
            return Err(anyhow::anyhow!("Chain ID is required for Ethereum transactions"));
        }

        // Validate gas values are numeric
        if let Some(gas_price) = &self.gas_price {
            gas_price.parse::<u64>().map_err(|_| anyhow::anyhow!("Invalid gas price format"))?;
        }
        if let Some(gas_limit) = &self.gas_limit {
            gas_limit.parse::<u64>().map_err(|_| anyhow::anyhow!("Invalid gas limit format"))?;
        }

        Ok(())
    }


    /// Get the transaction hash using SHA-256 with canonical encoding (V3)
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        // Version + domain separation
        hasher.update(b"WALLET_TX_V3\x00");

        // tx_type tag
        match &self.tx_type {
            TransactionType::NativeTransfer => hasher.update([0u8]),
            TransactionType::Erc20Transfer { token_address, token_amount } => {
                hasher.update([1u8]);
                // token_address
                hasher.update(b"|token_addr:");
                hasher.update(token_address.as_bytes());
                // token_amount (normalized integer)
                if let Ok(v) = Self::parse_u128_str_digits(token_amount) {
                    hasher.update(b"|token_amt:");
                    let be = Self::be_bytes_min_u128(v);
                    hasher.update((be.len() as u32).to_be_bytes());
                    hasher.update(&be);
                }
            }
            TransactionType::ContractCall { contract_address, data, value } => {
                hasher.update([2u8]);
                hasher.update(b"|contract:");
                hasher.update(contract_address.as_bytes());
                hasher.update(b"|calldata:");
                hasher.update((data.len() as u32).to_be_bytes());
                hasher.update(data);
                if let Some(val) = value {
                    if let Ok(v) = Self::parse_u128_str_digits(val) {
                        hasher.update(b"|value:");
                        let be = Self::be_bytes_min_u128(v);
                        hasher.update((be.len() as u32).to_be_bytes());
                        hasher.update(&be);
                    }
                }
            }
        }

        // Common fields with tags and normalized numeric encodings
        hasher.update(b"|to:");
        hasher.update(self.to.as_bytes());

        hasher.update(b"|amount:");
        if let Ok(v) = self.normalized_native_amount() {
            let be = Self::be_bytes_min_u128(v);
            hasher.update((be.len() as u32).to_be_bytes());
            hasher.update(&be);
        }

        hasher.update(b"|net:");
        hasher.update(self.network.as_bytes());

        if let Some(gas_price) = &self.gas_price {
            if let Ok(v) = gas_price.parse::<u64>() {
                hasher.update(b"|gas_price:");
                hasher.update(v.to_be_bytes());
            }
        }
        if let Some(gas_limit) = &self.gas_limit {
            if let Ok(v) = gas_limit.parse::<u64>() {
                hasher.update(b"|gas_limit:");
                hasher.update(v.to_be_bytes());
            }
        }
        if let Some(nonce) = self.nonce {
            hasher.update(b"|nonce:");
            hasher.update(nonce.to_be_bytes());
        }
        if let Some(chain_id) = self.chain_id {
            hasher.update(b"|chain_id:");
            hasher.update(chain_id.to_be_bytes());
        }
        if let Some(data) = &self.data {
            hasher.update(b"|data:");
            hasher.update((data.len() as u32).to_be_bytes());
            hasher.update(data);
        }

        // Timestamp kept in RFC3339 for readability; still deterministic for a given Tx
        hasher.update(b"|ts:");
        hasher.update(self.timestamp.to_rfc3339().as_bytes());

        format!("{:x}", hasher.finalize())
    }

    /// Legacy constructor for backward compatibility
    pub fn new(_w: &crate::mvp::Wallet, to: &str, amount: u64) -> Self {
        Self::new_native_transfer(to, &amount.to_string(), "unknown")
    }
}

/// Private key wrapper (32 bytes) with secrecy::Secret for automatic zeroization and display-hiding
pub struct PrivateKey(Secret<[u8; 32]>);
impl PrivateKey {
    pub fn new(k: [u8; 32]) -> Self {
        Self(Secret::new(k))
    }
    /// Expose the underlying bytes (read-only) when strictly necessary.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.expose_secret()
    }

    /// Scoped access to the underlying secret bytes. Prefer this over `as_bytes()` so
    /// callers can't accidentally hold on to or clone secret data outside a small scope.
    ///
    /// Example:
    ///   pk.with_secret(|b| { /* use b: &[u8;32] here only */ });
    pub fn with_secret<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[u8; 32]) -> R,
    {
        // Expose the secret to the closure for immediate use. This does not allocate a
        // new buffer and encourages scoped usage patterns.
        f(self.0.expose_secret())
    }

    /// Try to construct a PrivateKey from a byte slice (must be 32 bytes).
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, anyhow::Error> {
        if slice.len() != 32 {
            return Err(anyhow::anyhow!("Private key must be 32 bytes"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&slice[..32]);
        Ok(PrivateKey::new(arr))
    }
}
impl Zeroize for PrivateKey {
    fn zeroize(&mut self) {
        // overwrite the inner secret by replacing with a zeroed array
        let zero = [0u8; 32];
        // ensure previous content is dropped
        self.0 = Secret::new(zero);
    }
}
impl Drop for PrivateKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Public key wrapper (33 bytes)
pub struct PublicKey([u8; 33]);
impl PublicKey {
    pub fn new(k: [u8; 33]) -> Self {
        Self(k)
    }
}
impl Zeroize for PublicKey {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}
impl Drop for PublicKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Address wrapper (20 bytes)
pub struct Address([u8; 20]);
impl Address {
    pub fn new(a: [u8; 20]) -> Self {
        Self(a)
    }
}
impl Zeroize for Address {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}
impl Drop for Address {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Nonce wrapper (u64)
pub struct Nonce(u64);
impl Nonce {
    pub fn new(n: u64) -> Self {
        Self(n)
    }
    pub fn get(&self) -> u64 {
        self.0
    }
    pub fn set(&mut self, v: u64) {
        self.0 = v;
    }
}
impl Zeroize for Nonce {
    fn zeroize(&mut self) {
        self.0 = 0;
    }
}
impl Drop for Nonce {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_erc20_transfer_calldata_happy_path() {
        // Address: 0x1111111111111111111111111111111111111111
        let addr = "0x1111111111111111111111111111111111111111";
        let amount = "1000000000000000000"; // 1e18
        let data = Tx::encode_erc20_transfer_calldata(addr, amount).expect("calldata");

        // 4 (selector) + 32 (address) + 32 (amount)
        assert_eq!(data.len(), 68);
        // Selector a9059cbb
        assert_eq!(&data[0..4], &[0xa9, 0x05, 0x9c, 0xbb]);

        // Address should be left-padded to 32 bytes
        // First 12 should be zero
        assert!(data[4..4 + 12].iter().all(|&b| b == 0));
        // Next 20 bytes should equal the address bytes
        let mut expected_addr = [0u8; 20];
        let raw = &addr[2..];
        for i in 0..20 {
            expected_addr[i] = u8::from_str_radix(&raw[2 * i..2 * i + 2], 16).unwrap();
        }
        assert_eq!(&data[4 + 12..4 + 12 + 20], &expected_addr);

        // Amount should be 32-byte big-endian; first 16 bytes zero for this range
        assert!(data[4 + 32..4 + 32 + 16].iter().all(|&b| b == 0));
        let amt_tail = &data[4 + 32 + 16..4 + 32 + 32];
        let amt = amount.parse::<u128>().unwrap();
        assert_eq!(amt_tail, &amt.to_be_bytes());
    }

    #[test]
    fn test_encode_erc20_transfer_calldata_invalid_address() {
        // Too short
        let err = Tx::encode_erc20_transfer_calldata("0x1234", "1").unwrap_err();
        assert!(err.to_string().contains("Invalid ERC-20 address"));
        // Non-hex
        let err =
            Tx::encode_erc20_transfer_calldata("0xZZ11111111111111111111111111111111111111", "1")
                .unwrap_err();
        // Non-hex characters trigger the generic format error in the fast path
        assert!(err.to_string().contains("Invalid ERC-20 address"));
    }

    #[test]
    fn test_encode_erc20_transfer_calldata_without_0x_prefix() {
        // Same as happy path but without 0x prefix
        let addr = "1111111111111111111111111111111111111111";
        let amount = "42";
        let data = Tx::encode_erc20_transfer_calldata(addr, amount).expect("calldata");
        assert_eq!(&data[0..4], &[0xa9, 0x05, 0x9c, 0xbb]);
        // Check last 16 bytes encode 42
        let amt_tail = &data[4 + 32 + 16..4 + 32 + 32];
        assert_eq!(amt_tail, &42u128.to_be_bytes());
    }

    #[test]
    fn test_encode_erc20_transfer_calldata_invalid_amount() {
        let addr = "0x1111111111111111111111111111111111111111";
        let err = Tx::encode_erc20_transfer_calldata(addr, "1.23").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("integer") || msg.contains("Uint256 must be"));
    }

    #[test]
    fn test_wallet_from_mnemonic() {
        let wallet = crate::mvp::Wallet::from_mnemonic("test mnemonic").unwrap();
        let mn = wallet.mnemonic_secret();
        assert_eq!(mn.as_str(), "test mnemonic");
    }

    #[test]
    fn test_tx_new() {
        let wallet = crate::mvp::Wallet::from_mnemonic("test").unwrap();
        let tx = Tx::new(&wallet, "0x123", 100);
        assert_eq!(tx.to, "0x123");
        assert_eq!(tx.amount, "100");
        assert_eq!(tx.network, "unknown");
        assert_eq!(tx.tx_type, TransactionType::NativeTransfer);
    }

    #[test]
    fn test_tx_serialize() {
        let wallet = crate::mvp::Wallet::from_mnemonic("test").unwrap();
        let tx = Tx::new(&wallet, "0x123", 100);
        let serialized = tx.serialize().unwrap();
        assert!(!serialized.is_empty());
        // validate可以反序列化
        let deserialized: Tx = Tx::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.to, tx.to);
        assert_eq!(deserialized.amount, tx.amount);
        assert_eq!(deserialized.tx_type, tx.tx_type);
    }

    #[test]
    fn test_tx_new_native_transfer() {
        let tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth");
        assert_eq!(tx.to, "0x123");
        assert_eq!(tx.amount, "1000000000000000000");
        assert_eq!(tx.network, "eth");
        assert_eq!(tx.tx_type, TransactionType::NativeTransfer);
    }

    #[test]
    fn test_tx_new_erc20_transfer() {
        let tx = Tx::new_erc20_transfer("0xToken", "0x123", "1000000", "eth");
        assert_eq!(tx.to, "0x123");
        assert_eq!(tx.amount, "0");
        assert_eq!(tx.network, "eth");
        match tx.tx_type {
            TransactionType::Erc20Transfer { token_address, token_amount } => {
                assert_eq!(token_address, "0xToken");
                assert_eq!(token_amount, "1000000");
            }
            _ => panic!("Expected ERC20 transfer"),
        }
    }

    #[test]
    fn test_tx_new_contract_call() {
        let data = vec![0x12, 0x34, 0x56];
        let tx =
            Tx::new_contract_call("0xContract", data.clone(), Some("1000000000000000000"), "eth");
        assert_eq!(tx.to, "0xContract");
        assert_eq!(tx.amount, "1000000000000000000");
        assert_eq!(tx.network, "eth");
        assert_eq!(tx.data, Some(data));
        match tx.tx_type {
            TransactionType::ContractCall { contract_address, data: call_data, value } => {
                assert_eq!(contract_address, "0xContract");
                assert_eq!(call_data, vec![0x12, 0x34, 0x56]);
                assert_eq!(value, Some("1000000000000000000".to_string()));
            }
            _ => panic!("Expected contract call"),
        }
    }

    #[test]
    fn test_tx_with_gas() {
        let tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth")
            .with_gas("20000000000", "21000");
        assert_eq!(tx.gas_price, Some("20000000000".to_string()));
        assert_eq!(tx.gas_limit, Some("21000".to_string()));
    }

    #[test]
    fn test_tx_with_nonce() {
        let tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth").with_nonce(5);
        assert_eq!(tx.nonce, Some(5));
    }

    #[test]
    fn test_tx_with_chain_id() {
        let tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth").with_chain_id(1);
        assert_eq!(tx.chain_id, Some(1));
    }

    #[test]
    fn test_tx_validate_valid_ethereum() {
        let tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth")
            .with_gas("20000000000", "21000")
            .with_nonce(5)
            .with_chain_id(1);
        assert!(tx.validate().is_ok());
    }

    #[test]
    fn test_tx_validate_invalid_ethereum_missing_gas() {
        let tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth");
        assert!(tx.validate().is_err());
        assert!(tx.validate().unwrap_err().to_string().contains("Gas price is required"));
    }

    #[test]
    fn test_tx_validate_invalid_empty_to() {
        let tx = Tx::new_native_transfer("", "1000000000000000000", "eth")
            .with_gas("20000000000", "21000")
            .with_nonce(5)
            .with_chain_id(1);
        assert!(tx.validate().is_err());
        assert!(tx
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Recipient address cannot be empty"));
    }

    #[test]
    fn test_tx_validate_valid_polygon() {
        // Polygon是EVM链，需要gas参数
        let tx = Tx {
            tx_type: TransactionType::NativeTransfer,
            to: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            amount: "1000000".to_string(),
            network: "polygon".to_string(),
            gas_price: Some("20000000000".to_string()),
            gas_limit: Some("21000".to_string()),
            nonce: Some(0),
            chain_id: Some(137),
            data: None,
            timestamp: chrono::Utc::now(),
        };
        assert!(tx.validate().is_ok());
    }

    #[test]
    fn test_tx_validate_invalid_erc20() {
        let tx = Tx {
            tx_type: TransactionType::Erc20Transfer {
                token_address: "".to_string(),
                token_amount: "1000000".to_string(),
            },
            to: "0x123".to_string(),
            amount: "0".to_string(),
            network: "eth".to_string(),
            gas_price: Some("20000000000".to_string()),
            gas_limit: Some("21000".to_string()),
            data: None,
            nonce: Some(5),
            chain_id: Some(1),
            timestamp: chrono::Utc::now(),
        };
        assert!(tx.validate().is_err());
        assert!(tx.validate().unwrap_err().to_string().contains("Token address cannot be empty"));
    }

    #[test]
    fn test_tx_hash() {
        let tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth");
        let hash1 = tx.hash();
        let hash2 = tx.hash();
        assert_eq!(hash1, hash2); // Hash should be deterministic
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_tx_roundtrip_serialization() {
        let original_tx = Tx::new_native_transfer("0x123", "1000000000000000000", "eth")
            .with_gas("20000000000", "21000")
            .with_nonce(5)
            .with_chain_id(1);

        let serialized = original_tx.serialize().unwrap();
        let deserialized_tx = Tx::deserialize(&serialized).unwrap();

        assert_eq!(original_tx.tx_type, deserialized_tx.tx_type);
        assert_eq!(original_tx.to, deserialized_tx.to);
        assert_eq!(original_tx.amount, deserialized_tx.amount);
        assert_eq!(original_tx.network, deserialized_tx.network);
        assert_eq!(original_tx.gas_price, deserialized_tx.gas_price);
        assert_eq!(original_tx.gas_limit, deserialized_tx.gas_limit);
        assert_eq!(original_tx.nonce, deserialized_tx.nonce);
        assert_eq!(original_tx.chain_id, deserialized_tx.chain_id);
    }

    #[test]
    fn test_private_key_new() {
        let key = [1u8; 32];
        let pk = PrivateKey::new(key);
        assert_eq!(*pk.as_bytes(), key);
    }

    #[test]
    fn test_public_key_new() {
        let key = [2u8; 33];
        let pk = PublicKey::new(key);
        assert_eq!(pk.0, key);
    }

    #[test]
    fn test_address_new() {
        let addr = [3u8; 20];
        let address = Address::new(addr);
        assert_eq!(address.0, addr);
    }

    #[test]
    fn test_nonce_new() {
        let nonce = Nonce::new(42);
        assert_eq!(nonce.get(), 42);
    }

    #[test]
    fn test_nonce_set() {
        let mut nonce = Nonce::new(0);
        nonce.set(100);
        assert_eq!(nonce.get(), 100);
    }
}
