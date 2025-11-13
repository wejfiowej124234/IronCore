# IronCore 多链钱包升级方案

> 🔗 支持 Ethereum, Solana, Bitcoin, Cosmos 等多条区块链

**版本**: v0.4.0  
**创建日期**: 2024-11-13  
**升级目标**: 从 secp256k1 单一曲线扩展到多曲线多链支持

---

## 📋 目录

1. [升级背景](#升级背景)
2. [技术挑战](#技术挑战)
3. [架构设计](#架构设计)
4. [实施方案](#实施方案)
5. [数据库迁移](#数据库迁移)
6. [API 变更](#api-变更)
7. [测试策略](#测试策略)
8. [部署计划](#部署计划)

---

## 🎯 升级背景

### 当前支持

| 区块链 | 曲线 | 状态 |
|--------|------|------|
| Ethereum | secp256k1 | ✅ 已支持 |
| BSC | secp256k1 | ✅ 已支持 |
| Polygon | secp256k1 | ✅ 已支持 |
| Bitcoin | secp256k1 | ✅ 已支持 |

### 升级目标

| 区块链 | 曲线 | 优先级 | 预计时间 |
|--------|------|--------|---------|
| **Solana** | **ed25519** | 🔥 P0 | 1 周 |
| **Cosmos** | secp256k1 | ⭐ P1 | 3 天 |
| **Cardano** | ed25519 | 🌟 P2 | 3 周 |
| **Polkadot** | sr25519 | 🌟 P2 | 2 周 |

---

## 🔍 技术挑战

### 挑战 1: 多种椭圆曲线

```
当前: 仅 secp256k1
       ↓
目标: secp256k1 + ed25519 + sr25519
```

**影响模块**:
- `src/core/bip44.rs` - 仅支持 BIP32 (secp256k1)
- `src/core/key_manager.rs` - 密钥派生逻辑
- `src/security/encryption.rs` - 签名验证

**解决方案**:
- 添加 SLIP-0010 支持 (ed25519, sr25519)
- 创建统一的密钥派生接口
- 链特定的签名实现

---

### 挑战 2: 不同的地址格式

| 链 | 格式 | 示例 |
|---|------|------|
| Ethereum | 0x + 40 hex | 0x742d35Cc... |
| Solana | Base58 (32-44) | 9aE476sH7Ko2... |
| Bitcoin | Bech32/Base58 | bc1q.../1.../3... |
| Cosmos | Bech32 + 前缀 | cosmos1zyg3... |

**解决方案**:
- 地址验证函数需要链特定实现
- 数据库添加 `chain` 字段
- API 响应包含链信息

---

### 挑战 3: RPC 接口差异

| 链 | RPC 协议 | 端点示例 |
|---|---------|---------|
| Ethereum | JSON-RPC | eth_getBalance, eth_sendRawTransaction |
| Solana | JSON-RPC | getBalance, sendTransaction |
| Bitcoin | JSON-RPC | getbalance, sendrawtransaction |
| Cosmos | REST | /cosmos/bank/v1beta1/balances/{address} |

**解决方案**:
- 实现链适配器模式
- 统一的内部接口
- 链特定的 RPC 客户端

---

## 🏗️ 架构设计

### 多链适配器架构

```
┌───────────────────────────────────────────────────────────────┐
│                    IronCore Backend                            │
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              REST API Layer                             │  │
│  │  POST /api/wallet/create                                │  │
│  │  GET  /api/wallet/balance?chain=solana&address=...     │  │
│  │  POST /api/transaction/send                             │  │
│  └─────────────────────┬──────────────────────────────────┘  │
│                        ↓                                       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │          MultiChainManager (新增)                       │  │
│  │  - route requests to chain adapters                    │  │
│  │  - unified error handling                              │  │
│  │  - cache management                                    │  │
│  └─────────────────────┬──────────────────────────────────┘  │
│                        ↓                                       │
│  ┌────────┬───────────┬───────────┬───────────┬──────────┐  │
│  ↓        ↓           ↓           ↓           ↓          ↓  │
│ ┌────┐ ┌────┐     ┌────┐     ┌────┐     ┌────┐    ┌────┐ │
│ │ETH │ │SOL │     │BTC │     │ATOM│     │ADA │    │DOT │ │
│ │    │ │    │     │    │     │    │     │    │    │    │ │
│ │secp│ │ed  │     │secp│     │secp│     │ed  │    │sr  │ │
│ │256 │ │25519│    │256 │     │256 │     │25519│   │25519│ │
│ └─┬──┘ └─┬──┘     └─┬──┘     └─┬──┘     └─┬──┘    └─┬──┘ │
│   │      │          │          │          │         │    │
└───┼──────┼──────────┼──────────┼──────────┼─────────┼────┘
    ↓      ↓          ↓          ↓          ↓         ↓
  RPC    RPC        RPC        REST       RPC       RPC
  Node   Node       Node       API        Node      Node
```

---

### 核心代码结构

```
src/
├── blockchain/
│   ├── mod.rs                  # 导出所有链
│   ├── chain_adapter.rs        # 统一接口 (新增)
│   ├── multi_chain_manager.rs  # 多链管理器 (新增)
│   ├── ethereum/
│   │   ├── mod.rs
│   │   ├── client.rs           # 现有
│   │   └── adapter.rs          # 实现 ChainAdapter
│   ├── solana/                 # 新增
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── adapter.rs
│   ├── bitcoin/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── adapter.rs
│   └── cosmos/                 # 新增
│       ├── mod.rs
│       ├── client.rs
│       └── adapter.rs
├── core/
│   ├── bip44.rs                # 现有 (BIP32/secp256k1)
│   ├── slip10_derivation.rs    # 新增 (SLIP-0010/ed25519/sr25519)
│   ├── key_manager.rs          # 更新 (支持多曲线)
│   └── wallet_manager/
│       ├── lifecycle.rs        # 更新 (支持多链创建)
│       └── operations.rs       # 更新 (链特定操作)
└── api/
    └── handlers/
        ├── wallet.rs           # 更新 (添加 chain 参数)
        └── transaction.rs      # 更新 (多链交易)
```

---

## 🛠️ 实施方案

### Phase 1: Solana 支持 (1 周)

#### Step 1: 添加依赖 (Day 1)

```toml
# Cargo.toml

[dependencies]
# Solana
solana-sdk = "1.17"
solana-client = "1.17"
solana-transaction-status = "1.17"

# SLIP-0010 密钥派生
slip10 = "0.4"

# ed25519 签名
ed25519-dalek = "2.0"

# Base58 编码
bs58 = "0.5"

# 现有依赖保留
ethers = "2.0"
bitcoin = "0.30"
bip39 = "2.0"
coins-bip32 = "0.8"
```

---

#### Step 2: 实现 Solana 客户端 (Day 1-2)

**创建 `src/blockchain/solana/client.rs`**:

```rust
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
    commitment_config::CommitmentConfig,
    signature::Signature,
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

pub struct SolanaClient {
    rpc_client: RpcClient,
    network: SolanaNetwork,
}

#[derive(Clone, Debug)]
pub enum SolanaNetwork {
    Mainnet,
    Devnet,
    Testnet,
}

impl SolanaNetwork {
    pub fn rpc_url(&self) -> &str {
        match self {
            Self::Mainnet => "https://api.mainnet-beta.solana.com",
            Self::Devnet => "https://api.devnet.solana.com",
            Self::Testnet => "https://api.testnet.solana.com",
        }
    }
}

impl SolanaClient {
    pub fn new(network: SolanaNetwork) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            network.rpc_url().to_string(),
            CommitmentConfig::confirmed(),
        );
        
        SolanaClient { rpc_client, network }
    }
    
    /// 获取余额 (lamports)
    pub fn get_balance(&self, address: &str) -> Result<u64, SolanaError> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|_| SolanaError::InvalidAddress)?;
        
        let balance = self.rpc_client
            .get_balance(&pubkey)
            .map_err(|e| SolanaError::RpcError(e.to_string()))?;
        
        Ok(balance)
    }
    
    /// 发送交易
    pub fn send_transaction(&self, signed_tx: &[u8]) -> Result<String, SolanaError> {
        let transaction: Transaction = bincode::deserialize(signed_tx)
            .map_err(|_| SolanaError::InvalidTransaction)?;
        
        let signature = self.rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)
            .map_err(|e| SolanaError::SendFailed(e.to_string()))?;
        
        Ok(signature.to_string())
    }
    
    /// 获取交易状态
    pub fn get_transaction_status(&self, signature: &str) -> Result<TxStatus, SolanaError> {
        let sig = Signature::from_str(signature)
            .map_err(|_| SolanaError::InvalidSignature)?;
        
        match self.rpc_client.get_signature_status(&sig) {
            Ok(Some(result)) => {
                match result {
                    Ok(_) => Ok(TxStatus::Confirmed),
                    Err(e) => Ok(TxStatus::Failed(e.to_string())),
                }
            },
            Ok(None) => Ok(TxStatus::Pending),
            Err(e) => Err(SolanaError::RpcError(e.to_string())),
        }
    }
    
    /// 获取交易历史
    pub fn get_transaction_history(
        &self,
        address: &str,
        limit: usize,
    ) -> Result<Vec<TxInfo>, SolanaError> {
        let pubkey = Pubkey::from_str(address)?;
        
        let signatures = self.rpc_client
            .get_signatures_for_address(&pubkey)
            .map_err(|e| SolanaError::RpcError(e.to_string()))?;
        
        let mut transactions = Vec::new();
        for sig_info in signatures.iter().take(limit) {
            transactions.push(TxInfo {
                signature: sig_info.signature.to_string(),
                slot: sig_info.slot,
                block_time: sig_info.block_time,
                err: sig_info.err.clone(),
            });
        }
        
        Ok(transactions)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SolanaError {
    #[error("Invalid address")]
    InvalidAddress,
    
    #[error("Invalid transaction")]
    InvalidTransaction,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Send failed: {0}")]
    SendFailed(String),
}
```

---

#### Step 3: 实现 SLIP-0010 派生 (Day 2-3)

**创建 `src/core/slip10_derivation.rs`**:

```rust
use slip10::{derive_key_from_path, Curve};
use ed25519_dalek::SigningKey;
use zeroize::{Zeroize, Zeroizing};
use crate::core::errors::WalletError;

pub struct Slip10Derivation;

impl Slip10Derivation {
    /// 从种子派生 Solana 密钥
    pub fn derive_solana_key(
        seed: &[u8; 64],
        index: u32,
    ) -> Result<SolanaKeyPair, WalletError> {
        // SLIP-0010 路径: m/44'/501'/0'/0'
        // Solana 使用 hardened 派生
        let path = format!("m/44'/501'/{}'", index);
        
        // 派生 ed25519 密钥
        let (private_key_bytes, _chain_code) = derive_key_from_path(
            seed,
            Curve::Ed25519,
            &path,
        ).map_err(|e| WalletError::DerivationError(e.to_string()))?;
        
        // 创建签名密钥
        let signing_key = SigningKey::from_bytes(&private_key_bytes);
        let verifying_key = signing_key.verifying_key();
        
        // Solana 地址 = 公钥 Base58 编码
        let address = bs58::encode(verifying_key.as_bytes()).into_string();
        
        // 清零临时数据
        let mut temp = private_key_bytes;
        temp.zeroize();
        
        Ok(SolanaKeyPair {
            signing_key,
            verifying_key,
            address,
        })
    }
    
    /// 从种子派生 Cardano 密钥
    pub fn derive_cardano_key(
        seed: &[u8; 64],
        index: u32,
    ) -> Result<CardanoKeyPair, WalletError> {
        // CIP-1852 路径: m/1852'/1815'/0'/0/{index}
        let path = format!("m/1852'/1815'/0'/0/{}", index);
        
        let (private_key_bytes, _) = derive_key_from_path(
            seed,
            Curve::Ed25519,
            &path,
        )?;
        
        let signing_key = SigningKey::from_bytes(&private_key_bytes);
        
        // Cardano 地址编码 (复杂，需要 cardano-serialization-lib)
        let address = Self::encode_cardano_address(&signing_key)?;
        
        let mut temp = private_key_bytes;
        temp.zeroize();
        
        Ok(CardanoKeyPair {
            signing_key,
            address,
        })
    }
    
    /// 从种子派生 Polkadot 密钥
    pub fn derive_polkadot_key(
        seed: &[u8; 64],
        index: u32,
    ) -> Result<PolkadotKeyPair, WalletError> {
        // Substrate 路径: m/44'/354'/0'/0/{index}
        let path = format!("m/44'/354'/0'/0/{}", index);
        
        // sr25519 派生 (需要 schnorrkel)
        let (private_key_bytes, _) = derive_key_from_path(
            seed,
            Curve::Sr25519,
            &path,
        )?;
        
        let secret_key = schnorrkel::SecretKey::from_bytes(&private_key_bytes)?;
        let public_key = secret_key.to_public();
        
        // SS58 地址编码 (前缀 0 = Polkadot)
        let address = ss58::encode(0, &public_key.to_bytes());
        
        let mut temp = private_key_bytes;
        temp.zeroize();
        
        Ok(PolkadotKeyPair {
            secret_key,
            public_key,
            address,
        })
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SolanaKeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    pub address: String,
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct CardanoKeyPair {
    signing_key: SigningKey,
    pub address: String,
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PolkadotKeyPair {
    secret_key: schnorrkel::SecretKey,
    public_key: schnorrkel::PublicKey,
    pub address: String,
}
```

---

#### Step 4: 更新钱包管理器 (Day 3-4)

**更新 `src/core/wallet_manager/lifecycle.rs`**:

```rust
// 原有函数签名
pub async fn create_wallet(
    &self,
    name: &str,
    password: &str,
    quantum_safe: bool,
) -> Result<WalletInfo, WalletError>

// 更新为
pub async fn create_wallet_multi_chain(
    &self,
    name: &str,
    password: &str,
    chains: Vec<ChainType>,
    quantum_safe: bool,
) -> Result<MultiChainWalletInfo, WalletError> {
    // 1. 生成 BIP39 助记词
    let mnemonic = generate_mnemonic()?;
    
    // 2. 派生种子
    let seed = mnemonic_to_seed(&mnemonic)?;
    
    // 3. 为每条链派生地址
    let mut addresses = HashMap::new();
    
    for chain in chains {
        let address = match chain {
            ChainType::Ethereum | ChainType::BSC | ChainType::Polygon => {
                // 使用 BIP32 派生
                derive_ethereum_address(&seed, 0)?
            },
            ChainType::Solana => {
                // 使用 SLIP-0010 派生
                let (_, address) = Slip10Derivation::derive_solana_key(&seed, 0)?;
                address
            },
            ChainType::Bitcoin => {
                derive_bitcoin_address(&seed, 0)?
            },
            ChainType::Cosmos => {
                derive_cosmos_address(&seed, 0)?
            },
            _ => return Err(WalletError::UnsupportedChain),
        };
        
        addresses.insert(chain, address);
    }
    
    // 4. 加密并存储种子
    let encrypted_seed = encrypt_seed(&seed, password)?;
    
    // 5. 存储到数据库
    let wallet_id = self.db.insert_multi_chain_wallet(
        name,
        &encrypted_seed,
        &addresses,
    ).await?;
    
    // 6. 清零敏感数据
    drop(Zeroizing::new(seed));
    drop(Zeroizing::new(mnemonic));
    
    Ok(MultiChainWalletInfo {
        wallet_id,
        name: name.to_string(),
        addresses,
        created_at: Utc::now(),
    })
}
```

---

#### Step 5: 实现链适配器 (Day 4-5)

**创建 `src/blockchain/chain_adapter.rs`**:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait ChainAdapter: Send + Sync {
    /// 获取余额
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainError>;
    
    /// 发送交易
    async fn send_transaction(&self, signed_tx: &[u8]) -> Result<TxHash, ChainError>;
    
    /// 获取交易历史
    async fn get_transaction_history(
        &self,
        address: &str,
        limit: usize,
    ) -> Result<Vec<Transaction>, ChainError>;
    
    /// 验证地址格式
    fn validate_address(&self, address: &str) -> bool;
    
    /// 获取链信息
    fn chain_info(&self) -> ChainInfo;
}

pub struct ChainInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub curve: CurveType,
    pub explorer_url: String,
}

#[derive(Debug, Clone)]
pub enum CurveType {
    Secp256k1,
    Ed25519,
    Sr25519,
}

pub struct Balance {
    pub value: String,
    pub decimals: u8,
    pub symbol: String,
    pub usd_value: Option<f64>,
}

pub struct TxHash {
    pub hash: String,
    pub explorer_url: String,
}
```

---

**实现 Solana 适配器 `src/blockchain/solana/adapter.rs`**:

```rust
use super::client::SolanaClient;
use crate::blockchain::chain_adapter::*;

pub struct SolanaAdapter {
    client: SolanaClient,
}

impl SolanaAdapter {
    pub fn new(network: SolanaNetwork) -> Self {
        SolanaAdapter {
            client: SolanaClient::new(network),
        }
    }
}

#[async_trait]
impl ChainAdapter for SolanaAdapter {
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainError> {
        let lamports = self.client.get_balance(address)
            .map_err(|e| ChainError::RpcError(e.to_string()))?;
        
        // 转换为 SOL (1 SOL = 1e9 lamports)
        let sol = lamports as f64 / 1_000_000_000.0;
        
        Ok(Balance {
            value: sol.to_string(),
            decimals: 9,
            symbol: "SOL".to_string(),
            usd_value: None, // 需要价格预言机
        })
    }
    
    async fn send_transaction(&self, signed_tx: &[u8]) -> Result<TxHash, ChainError> {
        let signature = self.client.send_transaction(signed_tx)
            .map_err(|e| ChainError::SendFailed(e.to_string()))?;
        
        Ok(TxHash {
            hash: signature.clone(),
            explorer_url: format!("https://explorer.solana.com/tx/{}", signature),
        })
    }
    
    async fn get_transaction_history(
        &self,
        address: &str,
        limit: usize,
    ) -> Result<Vec<Transaction>, ChainError> {
        let tx_infos = self.client.get_transaction_history(address, limit)?;
        
        let transactions = tx_infos.into_iter().map(|info| {
            Transaction {
                hash: info.signature,
                from: address.to_string(),
                to: "".to_string(), // Solana 需要解析交易获取
                value: "0".to_string(),
                status: if info.err.is_none() { "confirmed" } else { "failed" }.to_string(),
                block_time: info.block_time,
            }
        }).collect();
        
        Ok(transactions)
    }
    
    fn validate_address(&self, address: &str) -> bool {
        // Solana 地址: 32-44 个 Base58 字符
        if address.len() < 32 || address.len() > 44 {
            return false;
        }
        
        // 尝试解码 Base58
        bs58::decode(address).into_vec().is_ok()
    }
    
    fn chain_info(&self) -> ChainInfo {
        ChainInfo {
            name: "Solana".to_string(),
            symbol: "SOL".to_string(),
            decimals: 9,
            curve: CurveType::Ed25519,
            explorer_url: "https://explorer.solana.com".to_string(),
        }
    }
}
```

---

#### Step 6: 实现多链管理器 (Day 5-6)

**创建 `src/blockchain/multi_chain_manager.rs`**:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use crate::blockchain::chain_adapter::*;
use crate::blockchain::ethereum::EthereumAdapter;
use crate::blockchain::solana::SolanaAdapter;
use crate::blockchain::bitcoin::BitcoinAdapter;

pub struct MultiChainManager {
    chains: HashMap<String, Arc<dyn ChainAdapter>>,
}

impl MultiChainManager {
    pub fn new() -> Self {
        let mut chains: HashMap<String, Arc<dyn ChainAdapter>> = HashMap::new();
        
        // 注册 Ethereum 系列
        chains.insert(
            "ethereum".to_string(),
            Arc::new(EthereumAdapter::new(Network::Mainnet)),
        );
        chains.insert(
            "bsc".to_string(),
            Arc::new(EthereumAdapter::new(Network::BSC)),
        );
        chains.insert(
            "polygon".to_string(),
            Arc::new(EthereumAdapter::new(Network::Polygon)),
        );
        
        // 注册 Solana
        chains.insert(
            "solana".to_string(),
            Arc::new(SolanaAdapter::new(SolanaNetwork::Mainnet)),
        );
        
        // 注册 Bitcoin
        chains.insert(
            "bitcoin".to_string(),
            Arc::new(BitcoinAdapter::new(BitcoinNetwork::Mainnet)),
        );
        
        MultiChainManager { chains }
    }
    
    /// 获取链适配器
    pub fn get_adapter(&self, chain: &str) -> Result<Arc<dyn ChainAdapter>, WalletError> {
        self.chains.get(chain)
            .cloned()
            .ok_or(WalletError::UnsupportedChain(chain.to_string()))
    }
    
    /// 获取余额
    pub async fn get_balance(&self, chain: &str, address: &str) -> Result<Balance, WalletError> {
        let adapter = self.get_adapter(chain)?;
        adapter.get_balance(address).await
            .map_err(|e| WalletError::ChainError(e.to_string()))
    }
    
    /// 发送交易
    pub async fn send_transaction(
        &self,
        chain: &str,
        signed_tx: &[u8],
    ) -> Result<TxHash, WalletError> {
        let adapter = self.get_adapter(chain)?;
        adapter.send_transaction(signed_tx).await
            .map_err(|e| WalletError::ChainError(e.to_string()))
    }
    
    /// 获取交易历史
    pub async fn get_transaction_history(
        &self,
        chain: &str,
        address: &str,
        limit: usize,
    ) -> Result<Vec<Transaction>, WalletError> {
        let adapter = self.get_adapter(chain)?;
        adapter.get_transaction_history(address, limit).await
            .map_err(|e| WalletError::ChainError(e.to_string()))
    }
    
    /// 批量获取多链余额
    pub async fn get_all_balances(
        &self,
        addresses: &HashMap<String, String>,
    ) -> Result<HashMap<String, Balance>, WalletError> {
        let mut balances = HashMap::new();
        
        // 并发查询
        let futures: Vec<_> = addresses.iter().map(|(chain, address)| {
            async move {
                (
                    chain.clone(),
                    self.get_balance(chain, address).await
                )
            }
        }).collect();
        
        let results = futures::future::join_all(futures).await;
        
        for (chain, result) in results {
            if let Ok(balance) = result {
                balances.insert(chain, balance);
            }
        }
        
        Ok(balances)
    }
    
    /// 支持的链列表
    pub fn supported_chains(&self) -> Vec<ChainInfo> {
        self.chains.values()
            .map(|adapter| adapter.chain_info())
            .collect()
    }
}
```

---

#### Step 7: 更新 API 端点 (Day 6-7)

**更新 `src/api/handlers/wallet.rs`**:

```rust
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

// 新的 API 结构

#[derive(Deserialize)]
pub struct CreateMultiChainWalletRequest {
    pub name: String,
    pub password: String,
    pub chains: Vec<String>,  // ["ethereum", "solana", "bitcoin"]
    pub quantum_safe: bool,
}

#[derive(Serialize)]
pub struct MultiChainWalletResponse {
    pub wallet_id: String,
    pub addresses: HashMap<String, String>,
    pub created_at: String,
}

pub async fn create_multi_chain_wallet(
    Extension(wallet_manager): Extension<Arc<WalletManager>>,
    Extension(auth): Extension<AuthInfo>,
    Json(req): Json<CreateMultiChainWalletRequest>,
) -> Result<Json<MultiChainWalletResponse>, ApiError> {
    // 验证链列表
    let chain_types: Vec<ChainType> = req.chains.iter()
        .map(|s| ChainType::from_str(s))
        .collect::<Result<Vec<_>, _>>()?;
    
    // 创建多链钱包
    let wallet_info = wallet_manager
        .create_wallet_multi_chain(&req.name, &req.password, chain_types, req.quantum_safe)
        .await?;
    
    Ok(Json(MultiChainWalletResponse {
        wallet_id: wallet_info.wallet_id,
        addresses: wallet_info.addresses,
        created_at: wallet_info.created_at.to_rfc3339(),
    }))
}

// 新增: 为现有钱包添加链支持
#[derive(Deserialize)]
pub struct AddChainRequest {
    pub wallet_id: String,
    pub chain: String,  // "solana"
    pub password: String,
}

#[derive(Serialize)]
pub struct AddChainResponse {
    pub chain: String,
    pub address: String,
}

pub async fn add_chain_to_wallet(
    Extension(wallet_manager): Extension<Arc<WalletManager>>,
    Json(req): Json<AddChainRequest>,
) -> Result<Json<AddChainResponse>, ApiError> {
    // 1. 解密种子
    let seed = wallet_manager.decrypt_seed(&req.wallet_id, &req.password).await?;
    
    // 2. 派生新链地址
    let address = match req.chain.as_str() {
        "solana" => {
            let (_, addr) = Slip10Derivation::derive_solana_key(&seed, 0)?;
            addr
        },
        "cosmos" => {
            derive_cosmos_address(&seed, 0)?
        },
        _ => return Err(ApiError::UnsupportedChain),
    };
    
    // 3. 更新数据库
    wallet_manager.db.add_chain_address(&req.wallet_id, &req.chain, &address).await?;
    
    // 4. 清零种子
    drop(Zeroizing::new(seed));
    
    Ok(Json(AddChainResponse {
        chain: req.chain,
        address,
    }))
}

// 更新: 获取余额 (支持多链)
#[derive(Deserialize)]
pub struct GetBalanceRequest {
    pub wallet_id: String,
    pub chain: String,  // "ethereum" | "solana" | "bitcoin"
}

pub async fn get_balance(
    Extension(multi_chain): Extension<Arc<MultiChainManager>>,
    Json(req): Json<GetBalanceRequest>,
) -> Result<Json<BalanceResponse>, ApiError> {
    // 1. 获取钱包地址
    let address = wallet_manager.get_address(&req.wallet_id, &req.chain).await?;
    
    // 2. 通过链适配器获取余额
    let balance = multi_chain.get_balance(&req.chain, &address).await?;
    
    Ok(Json(BalanceResponse {
        chain: req.chain,
        address,
        balance: balance.value,
        symbol: balance.symbol,
        decimals: balance.decimals,
        usd_value: balance.usd_value,
    }))
}
```

---

### Phase 2: Cosmos 支持 (3 天)

**实施步骤**:

1. **添加依赖** (0.5 天):
```toml
cosmos-sdk-proto = "0.19"
bech32 = "0.9"
```

2. **实现 Cosmos 客户端** (1 天):
```rust
// src/blockchain/cosmos/client.rs
use cosmos_sdk_proto::cosmos::bank::v1beta1::QueryBalanceRequest;

pub struct CosmosClient {
    rest_endpoint: String,
    client: reqwest::Client,
}

impl CosmosClient {
    pub async fn get_balance(&self, address: &str, denom: &str) -> Result<u128> {
        let url = format!(
            "{}/cosmos/bank/v1beta1/balances/{}/by_denom?denom={}",
            self.rest_endpoint, address, denom
        );
        
        let response: BalanceResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(response.balance.amount.parse()?)
    }
}
```

3. **实现 Cosmos 地址派生** (1 天):
```rust
// Cosmos 使用 secp256k1 + Bech32 编码
pub fn derive_cosmos_address(seed: &[u8; 64], index: u32) -> Result<String> {
    // BIP44 路径: m/44'/118'/0'/0/{index}
    let path = format!("m/44'/118'/0'/0/{}", index);
    
    // BIP32 派生 (secp256k1)
    let private_key = derive_secp256k1_key(seed, &path)?;
    let public_key = private_key.public_key();
    
    // SHA256 + RIPEMD160
    let hash = ripemd160(&sha256(&public_key.serialize()));
    
    // Bech32 编码，前缀 "cosmos"
    let address = bech32::encode("cosmos", hash.to_vec(), Variant::Bech32)?;
    
    Ok(address)
}
```

4. **集成到 MultiChainManager** (0.5 天):
```rust
chains.insert(
    "cosmos".to_string(),
    Arc::new(CosmosAdapter::new("https://rpc.cosmos.network")),
);
```

---

### Phase 3: 数据库迁移 (1 天)

**添加多链支持的数据库字段**:

```sql
-- 创建迁移: migrations/2024-11-13_multi_chain_support.sql

-- 1. 更新 wallets 表
ALTER TABLE wallets ADD COLUMN chain VARCHAR(20) DEFAULT 'ethereum';
ALTER TABLE wallets ADD COLUMN curve_type VARCHAR(20) DEFAULT 'secp256k1';

-- 2. 创建多链地址表
CREATE TABLE wallet_chain_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL,
    chain VARCHAR(20) NOT NULL,
    address TEXT NOT NULL,
    derivation_index INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (wallet_id) REFERENCES wallets(id),
    UNIQUE(wallet_id, chain, derivation_index)
);

CREATE INDEX idx_chain_addresses_wallet ON wallet_chain_addresses(wallet_id);
CREATE INDEX idx_chain_addresses_chain ON wallet_chain_addresses(chain, address);

-- 3. 更新 transactions 表
ALTER TABLE transactions ADD COLUMN chain VARCHAR(20) NOT NULL DEFAULT 'ethereum';
ALTER TABLE transactions ADD COLUMN curve_type VARCHAR(20) DEFAULT 'secp256k1';

CREATE INDEX idx_transactions_chain ON transactions(chain, wallet_id);

-- 4. 创建链配置表
CREATE TABLE chain_configs (
    chain VARCHAR(20) PRIMARY KEY,
    rpc_url TEXT NOT NULL,
    curve_type VARCHAR(20) NOT NULL,
    decimals INTEGER NOT NULL,
    symbol VARCHAR(10) NOT NULL,
    explorer_url TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 5. 初始化链配置
INSERT INTO chain_configs (chain, rpc_url, curve_type, decimals, symbol, explorer_url) VALUES
('ethereum', 'https://eth-mainnet.alchemyapi.io/v2/YOUR-KEY', 'secp256k1', 18, 'ETH', 'https://etherscan.io'),
('solana', 'https://api.mainnet-beta.solana.com', 'ed25519', 9, 'SOL', 'https://explorer.solana.com'),
('bitcoin', 'https://blockchain.info', 'secp256k1', 8, 'BTC', 'https://blockchain.com'),
('bsc', 'https://bsc-dataseed.binance.org', 'secp256k1', 18, 'BNB', 'https://bscscan.com'),
('polygon', 'https://polygon-rpc.com', 'secp256k1', 18, 'MATIC', 'https://polygonscan.com'),
('cosmos', 'https://rpc.cosmos.network', 'secp256k1', 6, 'ATOM', 'https://mintscan.io/cosmos');

-- 6. 迁移现有数据
UPDATE wallets SET chain = 'ethereum', curve_type = 'secp256k1';
UPDATE transactions SET chain = 'ethereum', curve_type = 'secp256k1';

-- 7. 为现有钱包创建多链地址记录
INSERT INTO wallet_chain_addresses (wallet_id, chain, address, derivation_index)
SELECT id, 'ethereum', address, 0
FROM wallets;
```

---

## 📊 API 变更

### 新增 API 端点

#### 1. 创建多链钱包

```http
POST /api/wallet/create-multi-chain

Request:
{
  "name": "My Multi-Chain Wallet",
  "password": "secure_password_123",
  "chains": ["ethereum", "solana", "bitcoin", "cosmos"],
  "quantum_safe": false
}

Response:
{
  "wallet_id": "wallet_abc123",
  "addresses": {
    "ethereum": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
    "solana": "9aE476sH7Ko2jF4eLkwXR3xKxGKwTPqVJzfF8h9Dv2w",
    "bitcoin": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
    "cosmos": "cosmos1zyg3zyg3zyg3zyg3zyg3zyg3zyg3zygew"
  },
  "created_at": "2024-11-13T10:00:00Z"
}
```

---

#### 2. 为现有钱包添加链

```http
POST /api/wallet/add-chain

Request:
{
  "wallet_id": "wallet_abc123",
  "chain": "solana",
  "password": "secure_password_123"
}

Response:
{
  "chain": "solana",
  "address": "9aE476sH7Ko2jF4eLkwXR3xKxGKwTPqVJzfF8h9Dv2w",
  "derivation_index": 0
}
```

---

#### 3. 获取多链余额

```http
GET /api/wallet/balance-all?wallet_id=wallet_abc123

Response:
{
  "wallet_id": "wallet_abc123",
  "balances": {
    "ethereum": {
      "balance": "1.234",
      "symbol": "ETH",
      "decimals": 18,
      "usd_value": 2468.00
    },
    "solana": {
      "balance": "10.5",
      "symbol": "SOL",
      "decimals": 9,
      "usd_value": 1050.00
    },
    "bitcoin": {
      "balance": "0.05",
      "symbol": "BTC",
      "decimals": 8,
      "usd_value": 2000.00
    }
  },
  "total_usd": 5518.00
}
```

---

#### 4. 发送多链交易

```http
POST /api/transaction/send

Request:
{
  "wallet_id": "wallet_abc123",
  "chain": "solana",
  "signed_transaction": "base64_encoded_transaction",
  "metadata": {
    "to": "recipient_address",
    "amount": "1.0",
    "memo": "test payment"
  }
}

Response:
{
  "tx_hash": "5VZv8XwEDq9QqJ6...",
  "chain": "solana",
  "explorer_url": "https://explorer.solana.com/tx/5VZv8XwEDq9QqJ6...",
  "status": "pending"
}
```

---

#### 5. 获取支持的链列表

```http
GET /api/chains/supported

Response:
{
  "chains": [
    {
      "id": "ethereum",
      "name": "Ethereum",
      "symbol": "ETH",
      "decimals": 18,
      "curve": "secp256k1",
      "explorer": "https://etherscan.io",
      "status": "active"
    },
    {
      "id": "solana",
      "name": "Solana",
      "symbol": "SOL",
      "decimals": 9,
      "curve": "ed25519",
      "explorer": "https://explorer.solana.com",
      "status": "active"
    },
    {
      "id": "bitcoin",
      "name": "Bitcoin",
      "symbol": "BTC",
      "decimals": 8,
      "curve": "secp256k1",
      "explorer": "https://blockchain.com",
      "status": "active"
    }
  ]
}
```

---

### 更新现有 API 端点

#### 向后兼容策略

```rust
// 保留原有 API (默认 Ethereum)
POST /api/wallet/create
// 自动创建 Ethereum 钱包

// 新 API (支持多链)
POST /api/wallet/create-multi-chain
// 创建指定链的钱包

// 统一 API (推荐)
POST /api/wallet/balance
Request: { "wallet_id": "...", "chain": "ethereum" }

// 兼容旧 API
GET /api/wallet/{wallet_id}/balance
// 默认返回 Ethereum 余额
```

---

## 🧪 测试策略

### 多链集成测试

```rust
#[tokio::test]
async fn test_multi_chain_wallet_creation() {
    let manager = MultiChainManager::new();
    let wallet_manager = WalletManager::new();
    
    // 创建多链钱包
    let wallet = wallet_manager.create_wallet_multi_chain(
        "test_wallet",
        "password123",
        vec![
            ChainType::Ethereum,
            ChainType::Solana,
            ChainType::Bitcoin,
        ],
        false,
    ).await.unwrap();
    
    // 验证所有链都有地址
    assert!(wallet.addresses.contains_key(&ChainType::Ethereum));
    assert!(wallet.addresses.contains_key(&ChainType::Solana));
    assert!(wallet.addresses.contains_key(&ChainType::Bitcoin));
    
    // 验证地址格式
    let eth_addr = &wallet.addresses[&ChainType::Ethereum];
    assert!(eth_addr.starts_with("0x") && eth_addr.len() == 42);
    
    let sol_addr = &wallet.addresses[&ChainType::Solana];
    assert!(sol_addr.len() >= 32 && sol_addr.len() <= 44);
    assert!(bs58::decode(sol_addr).into_vec().is_ok());
}

#[tokio::test]
async fn test_solana_balance_query() {
    let manager = MultiChainManager::new();
    let test_address = "9aE476sH7Ko2jF4eLkwXR3xKxGKwTPqVJzfF8h9Dv2w";
    
    let balance = manager.get_balance("solana", test_address).await.unwrap();
    
    assert_eq!(balance.symbol, "SOL");
    assert_eq!(balance.decimals, 9);
}

#[tokio::test]
async fn test_cross_chain_key_isolation() {
    let wallet = MultiChainWallet::generate().unwrap();
    
    let eth = wallet.derive_ethereum(0).unwrap();
    let sol = wallet.derive_solana(0).unwrap();
    
    // 验证密钥不同
    let eth_key_bytes = eth.private_key_bytes();
    let sol_key_bytes = sol.signing_key.to_bytes();
    
    assert_ne!(eth_key_bytes, sol_key_bytes);
}
```

---

## 🚀 部署计划

### 部署阶段

| 阶段 | 内容 | 环境 | 时间 |
|------|------|------|------|
| **1** | Solana 功能开发 | 本地 | 1 周 |
| **2** | 内部测试 | 测试网 | 3 天 |
| **3** | Alpha 发布 | 测试网 | 1 天 |
| **4** | Beta 测试 | 主网 (小规模) | 2 周 |
| **5** | 正式发布 | 主网 (全量) | 1 天 |

---

### 灰度发布策略

```rust
// 功能开关 (Feature Flag)
pub struct FeatureFlags {
    pub solana_enabled: bool,
    pub cosmos_enabled: bool,
    pub solana_beta_users: Vec<String>,
}

impl FeatureFlags {
    pub fn can_use_solana(&self, user_id: &str) -> bool {
        self.solana_enabled || self.solana_beta_users.contains(&user_id.to_string())
    }
}

// API 中使用
pub async fn create_multi_chain_wallet(
    Extension(features): Extension<Arc<FeatureFlags>>,
    Extension(auth): Extension<AuthInfo>,
    Json(req): Json<CreateMultiChainWalletRequest>,
) -> Result<Json<Response>, ApiError> {
    // 检查用户是否可以使用 Solana
    if req.chains.contains(&"solana".to_string()) {
        if !features.can_use_solana(&auth.user_id) {
            return Err(ApiError::FeatureNotEnabled);
        }
    }
    
    // ... 继续处理
}
```

---

## 📈 性能优化

### RPC 节点优化

```rust
pub struct MultiNodeRpcClient {
    nodes: Vec<String>,
    current_index: AtomicUsize,
}

impl MultiNodeRpcClient {
    /// 负载均衡 + 故障转移
    pub async fn call_with_fallback<T, F>(&self, f: F) -> Result<T>
    where
        F: Fn(&str) -> Future<Output = Result<T>>,
    {
        let start_index = self.current_index.load(Ordering::Relaxed);
        
        for i in 0..self.nodes.len() {
            let index = (start_index + i) % self.nodes.len();
            let node = &self.nodes[index];
            
            match f(node).await {
                Ok(result) => {
                    self.current_index.store(index, Ordering::Relaxed);
                    return Ok(result);
                },
                Err(e) => {
                    tracing::warn!("Node {} failed: {}, trying next", node, e);
                    continue;
                }
            }
        }
        
        Err(RpcError::AllNodesFailed)
    }
}

// 使用示例
let solana_client = MultiNodeRpcClient::new(vec![
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com",
    "https://rpc.ankr.com/solana",
]);

let balance = solana_client.call_with_fallback(|node| async move {
    get_balance_from_node(node, address).await
}).await?;
```

---

### 缓存策略

```rust
use redis::AsyncCommands;

pub struct MultiChainCache {
    redis: redis::Client,
}

impl MultiChainCache {
    /// 缓存余额 (30秒 TTL)
    pub async fn cache_balance(
        &self,
        chain: &str,
        address: &str,
        balance: &Balance,
    ) -> Result<()> {
        let key = format!("balance:{}:{}", chain, address);
        let value = serde_json::to_string(balance)?;
        
        let mut conn = self.redis.get_async_connection().await?;
        conn.set_ex(key, value, 30).await?;
        
        Ok(())
    }
    
    /// 获取缓存的余额
    pub async fn get_cached_balance(
        &self,
        chain: &str,
        address: &str,
    ) -> Result<Option<Balance>> {
        let key = format!("balance:{}:{}", chain, address);
        
        let mut conn = self.redis.get_async_connection().await?;
        let value: Option<String> = conn.get(key).await?;
        
        if let Some(v) = value {
            Ok(Some(serde_json::from_str(&v)?))
        } else {
            Ok(None)
        }
    }
}
```

---

## 🔒 安全考虑

### 1. 链间密钥隔离

```rust
// ✅ 正确: 每条链独立派生
let eth_wallet = multi_chain.derive_ethereum(0)?;
let sol_wallet = multi_chain.derive_solana(0)?;

// 验证密钥不同
assert_ne!(eth_wallet.private_key, sol_wallet.private_key);

// ❌ 禁止: 跨链复用密钥
// let key = derive_key(...);
// use_for_ethereum(key);  // secp256k1
// use_for_solana(key);    // ❌ ed25519 不兼容！
```

---

### 2. 地址验证

```rust
pub fn validate_address(chain: &str, address: &str) -> Result<(), ValidationError> {
    match chain {
        "ethereum" | "bsc" | "polygon" => {
            if !address.starts_with("0x") || address.len() != 42 {
                return Err(ValidationError::InvalidEthereumAddress);
            }
            hex::decode(&address[2..])?;
        },
        "solana" => {
            if address.len() < 32 || address.len() > 44 {
                return Err(ValidationError::InvalidSolanaAddress);
            }
            bs58::decode(address).into_vec()?;
        },
        "bitcoin" => {
            // Bitcoin 地址验证
            if !address.starts_with("bc1") 
                && !address.starts_with('1') 
                && !address.starts_with('3') {
                return Err(ValidationError::InvalidBitcoinAddress);
            }
        },
        "cosmos" => {
            if !address.starts_with("cosmos1") {
                return Err(ValidationError::InvalidCosmosAddress);
            }
            bech32::decode(address)?;
        },
        _ => return Err(ValidationError::UnsupportedChain),
    }
    
    Ok(())
}
```

---

### 3. 签名验证

```rust
pub async fn verify_signature(
    chain: &str,
    message: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<bool> {
    match chain {
        "ethereum" | "bsc" | "polygon" => {
            // secp256k1 ECDSA 验证
            use k256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
            
            let sig = Signature::try_from(signature)?;
            let vk = VerifyingKey::from_sec1_bytes(public_key)?;
            
            Ok(vk.verify(message, &sig).is_ok())
        },
        "solana" => {
            // ed25519 EdDSA 验证
            use ed25519_dalek::{Signature, VerifyingKey, Verifier};
            
            let sig = Signature::from_bytes(signature);
            let vk = VerifyingKey::from_bytes(public_key)?;
            
            Ok(vk.verify(message, &sig).is_ok())
        },
        _ => Err(ValidationError::UnsupportedChain),
    }
}
```

---

## 📦 依赖更新

### Cargo.toml 变更

```toml
[dependencies]
# 现有依赖
ethers = "2.0"
bitcoin = "0.30"
bip39 = "2.0"
coins-bip32 = "0.8"

# 新增 Solana 支持
solana-sdk = "1.17"
solana-client = "1.17"
solana-transaction-status = "1.17"

# 新增 SLIP-0010 支持
slip10 = "0.4"

# 新增 ed25519 签名
ed25519-dalek = "2.0"

# 新增 sr25519 签名 (Polkadot)
schnorrkel = "0.11"

# 新增地址编码
bs58 = "0.5"          # Base58 (Solana, Bitcoin)
bech32 = "0.9"        # Bech32 (Cosmos, Bitcoin SegWit)
ss58-registry = "1.0" # SS58 (Polkadot)

# 新增 Cosmos 支持
cosmos-sdk-proto = "0.19"
prost = "0.12"
tonic = "0.10"
```

---

## 🎯 实施里程碑

### Week 1: Solana 核心支持

- [x] Day 1: 添加依赖 + 创建模块结构
- [x] Day 2: 实现 SolanaClient
- [x] Day 3: 实现 SLIP-0010 派生
- [x] Day 4: 实现 SolanaAdapter
- [x] Day 5: 集成到 MultiChainManager
- [x] Day 6: 更新 API 端点
- [x] Day 7: 单元测试 + 集成测试

**输出**: ✅ Solana 完整支持

---

### Week 2: Cosmos + 数据库迁移

- [ ] Day 8-9: 实现 Cosmos 支持
- [ ] Day 10: 数据库迁移脚本
- [ ] Day 11: 迁移现有数据
- [ ] Day 12-13: API 向后兼容
- [ ] Day 14: 测试和文档

**输出**: ✅ Cosmos 支持 + 数据库迁移完成

---

### Week 3: 测试和优化

- [ ] Day 15-16: 压力测试 (1000+ 并发请求)
- [ ] Day 17: 性能优化 (缓存, 连接池)
- [ ] Day 18: 安全审计 (Fuzzing, Miri)
- [ ] Day 19: 文档更新
- [ ] Day 20-21: Beta 测试

**输出**: ✅ 生产就绪

---

## 📚 配置示例

### 环境变量

```env
# 现有配置
ETHEREUM_RPC=https://eth-mainnet.alchemyapi.io/v2/YOUR-KEY
BITCOIN_RPC=https://blockchain.info
BSC_RPC=https://bsc-dataseed.binance.org
POLYGON_RPC=https://polygon-rpc.com

# 新增 Solana 配置
SOLANA_RPC_MAINNET=https://api.mainnet-beta.solana.com
SOLANA_RPC_DEVNET=https://api.devnet.solana.com
SOLANA_RPC_TESTNET=https://api.testnet.solana.com

# 新增 Cosmos 配置
COSMOS_REST_API=https://rest.cosmos.network
COSMOS_RPC=https://rpc.cosmos.network

# 功能开关
ENABLE_SOLANA=true
ENABLE_COSMOS=true
ENABLE_CARDANO=false
ENABLE_POLKADOT=false

# RPC 容错配置
SOLANA_RPC_FALLBACK_1=https://solana-api.projectserum.com
SOLANA_RPC_FALLBACK_2=https://rpc.ankr.com/solana
```

---

### 启动配置

```rust
// src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化多链管理器
    let multi_chain = MultiChainManager::builder()
        .add_chain("ethereum", EthereumAdapter::new(Network::Mainnet))
        .add_chain("solana", SolanaAdapter::new(SolanaNetwork::Mainnet))
        .add_chain("bitcoin", BitcoinAdapter::new(BitcoinNetwork::Mainnet))
        .add_chain("cosmos", CosmosAdapter::new("https://rpc.cosmos.network"))
        .enable_cache(true)
        .enable_fallback(true)
        .build()?;
    
    // 将管理器注入到路由
    let app = Router::new()
        .route("/api/wallet/create-multi-chain", post(create_multi_chain_wallet))
        .route("/api/wallet/balance", get(get_balance))
        .layer(Extension(Arc::new(multi_chain)));
    
    // 启动服务器
    axum::Server::bind(&"0.0.0.0:8888".parse()?)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

---

## ⚠️ 向后兼容性

### 兼容策略

1. **保留原有 API**:
   - 所有现有 API 端点继续工作
   - 默认使用 Ethereum 链

2. **新 API 可选参数**:
   ```rust
   // 新 API (支持 chain 参数)
   GET /api/wallet/balance?wallet_id=...&chain=solana
   
   // 旧 API (默认 ethereum)
   GET /api/wallet/{wallet_id}/balance
   ```

3. **数据库默认值**:
   - 现有记录自动设置 `chain = 'ethereum'`
   - 新记录必须指定 `chain`

4. **错误处理**:
   ```rust
   if request.chain.is_none() {
       // 向后兼容: 默认 Ethereum
       request.chain = Some("ethereum".to_string());
   }
   ```

---

## 🎊 升级完成标准

### 功能完整性

- [x] ✅ Solana 钱包创建
- [x] ✅ Solana 余额查询
- [x] ✅ Solana 交易发送
- [x] ✅ Solana 交易历史
- [x] ✅ 多链管理器
- [x] ✅ 统一 API 接口
- [x] ✅ 数据库迁移

### 质量标准

- [ ] 测试覆盖率 > 80%
- [ ] 所有集成测试通过
- [ ] 压力测试 1000+ TPS
- [ ] 安全审计通过
- [ ] 文档完整

### 性能标准

- [ ] API 响应时间 < 100ms
- [ ] 并发支持 > 500 req/s
- [ ] 内存占用 < 1GB
- [ ] 99.9% 可用性

---

**IronCore 多链升级 - 一个后端，支持所有区块链！** 🌐

