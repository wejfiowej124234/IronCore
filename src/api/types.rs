use serde::{Deserialize, Serialize};

/// queryaddress请求参数
#[derive(Debug, Deserialize)]
pub struct AddressQuery {
    /// Network type（ethereum, bitcoin等）- 可选，默认为eth
    pub network: Option<String>,
}

/// address响应
#[derive(Debug, Serialize)]
pub struct AddressResponse {
    /// 区块链address
    pub address: String,
    /// Network type
    pub network: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateWalletRequest {
    /// Wallet name（必填，字母数字下划线，唯一）
    pub name: String,
    /// Password（可选，旧版本兼容，非托管模式不需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// walletaddress（非托管模式：前端生成并传入）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    /// 是否启用量子安全（可选，默认false）
    #[serde(default)]
    pub quantum_safe: bool,
    /// 是否生成mnemonic（可选，默认true，非托管模式由前端生成）
    #[serde(default = "default_true")]
    pub generate_mnemonic: bool,
    /// mnemonic单词数量（可选，12或24，默认12）
    #[serde(default = "default_mnemonic_count")]
    pub mnemonic_word_count: u32,
    /// wallet类型（可选："standard", "quantum", "multisig", "hardware"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_type: Option<String>,
    /// 多签配置（仅当wallet_type为"multisig"时需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multisig_config: Option<MultiSigConfig>,
}

/// 多签wallet配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigConfig {
    /// 需要的sign数（M）
    pub m: u32,
    /// 总sign者数（N）
    pub n: u32,
    /// sign者列表
    pub signers: Vec<SignerInfo>,
}

/// sign者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerInfo {
    /// sign者address
    pub address: String,
    /// sign者名称（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct WalletResponse {
    /// walletID
    pub id: String,
    /// Wallet name
    pub name: String,
    /// walletaddress
    pub address: String,
    /// 是否启用量子安全
    pub quantum_safe: bool,
    /// wallet类型（standard, quantum, multisig, hardware）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_type: Option<String>,
    /// mnemonic（仅在创建时返回一次，之后永不返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic: Option<String>,
    /// Warning信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

// 默认值辅助函数
fn default_true() -> bool {
    true
}

fn default_mnemonic_count() -> u32 {
    12
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SendTransactionRequest {
    /// 目标address（优先使用 to，兼容 to_address）
    #[serde(alias = "to_address")]
    pub to: String,
    pub amount: String,
    pub network: String,
    /// Password（托管模式使用，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 已Sign transaction（非托管模式：前端sign后传入）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_tx: Option<String>,
    /// 幂等键（可选，防止重复提交）
    #[serde(skip_serializing_if = "Option::is_none", alias = "clientRequestId")]
    pub client_request_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionResponse {
    /// transactionID（前端期望）
    pub tx_id: String,
    /// Transaction hash（兼容字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    pub status: String,
    /// network（新增）
    pub network: String,
    /// 时间戳（ISO8601 UTC）
    pub timestamp: String,
    /// 手续费（字符串格式）
    pub fee: String,
    /// 确认数（字符串格式）
    pub confirmations: String,
}

#[derive(Serialize, Deserialize)]
pub struct SendTransactionResponse {
    pub tx_hash: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BridgeAssetsRequest {
    pub from_wallet: String,
    pub from_chain: String,
    pub to_chain: String,
    pub token: String,
    pub amount: String,
    /// 幂等键（可选，防止重复提交）
    #[serde(skip_serializing_if = "Option::is_none", alias = "clientRequestId")]
    pub client_request_id: Option<String>,
}

#[derive(Serialize)]
pub struct BridgeResponse {
    /// 桥接ID（前端期望）
    pub bridge_id: String,
    /// 桥接transactionID（兼容字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_tx_id: Option<String>,
    /// 状态（新增，前端需要）
    pub status: String,
    /// 目标链（前端期望）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_chain: Option<String>,
    /// 金额（前端期望）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// 源链（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_chain: Option<String>,
    /// 代币（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Serialize)]
pub struct BridgeTransactionResponse {
    pub id: String,
    pub from_wallet: String,
    pub from_chain: String,
    pub to_chain: String,
    pub token: String,
    pub amount: String,
    pub status: String,
    pub source_tx_hash: Option<String>,
    pub destination_tx_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub fee_amount: Option<String>,
    pub estimated_completion_time: Option<String>,
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: String,
    pub network: String,
    pub symbol: String,
}

#[derive(Serialize)]
pub struct TransactionHistoryResponse {
    pub transactions: Vec<crate::blockchain::traits::TransactionInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedBackupResponse {
    /// Format version for the encrypted backup object
    pub version: String,
    /// Algorithm used, e.g. AES-256-GCM
    pub alg: String,
    /// KEK identifier used to encrypt this backup (optional)
    pub kek_id: Option<String>,
    /// Base64-encoded nonce
    pub nonce: String,
    /// Base64-encoded ciphertext (encrypted seed phrase)
    pub ciphertext: String,
    /// Wallet name for reference
    pub wallet: String,
}

// Backwards-compatible alias for handler usage in tests; production handlers should
// return `EncryptedBackupResponse`. For test-env, we still allow returning plaintext
// in the `ciphertext` field with `alg = "PLAINTEXT"` to preserve deterministic tests.
pub type BackupResponse = EncryptedBackupResponse;

#[derive(Clone, Debug, Deserialize)]
pub struct RestoreWalletRequest {
    pub name: String,
    pub seed_phrase: String,
    #[serde(default)]
    pub quantum_safe: bool,
    /// BIP39Password短语（可选，用于额外保护）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    /// 指定network（可选："eth" | "btc" | "bsc" | "polygon"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// 自定义派生路径（可选，如 m/44'/60'/0'/0）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation_path: Option<String>,
    /// address类型（可选："btc_bech32" | "btc_p2sh" | "btc_legacy" | "evm"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_type: Option<String>,
    /// 起始索引（可选，默认0）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<u32>,
    /// 批量导入数量（可选，默认1）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_count: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultiSigTransactionRequest {
    /// 目标address（优先使用 to，兼容 to_address）
    #[serde(alias = "to_address")]
    pub to: String,
    pub amount: String,
    pub network: String,
    pub signatures: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiSigTransactionResponse {
    pub tx_hash: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RotateSigningKeyResponse {
    pub wallet: String,
    pub old_version: u32,
    pub new_version: u32,
}
