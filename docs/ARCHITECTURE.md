# 🏗️ Architecture Documentation

## System Overview

This document provides a comprehensive overview of the blockchain wallet's architecture, design decisions, and implementation patterns.

---

## 📊 High-Level Architecture

### System Layers

```
┌─────────────────────────────────────────────────────────┐
│                    Client Layer                          │
│  React App (TypeScript) + Material-UI                   │
│  - Wallet Management    - Transactions                  │
│  - Settings             - Risk Detection UI             │
│  - Multi-chain Support  - Cross-chain Bridge            │
└────────────────┬────────────────────────────────────────┘
                 │ HTTPS/REST API (30+ endpoints)
                 │ WebSocket (real-time events)
┌────────────────▼────────────────────────────────────────┐
│                   API Gateway (Axum)                     │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Middleware Stack                                │  │
│  │  - JWT Authentication                            │  │
│  │  - Rate Limiting (governor)                      │  │
│  │  - CORS Handling                                 │  │
│  │  - Request Validation                            │  │
│  │  - Error Handling                                │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────┬────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────┐
│              Business Logic Layer                        │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   Wallet     │  │ Transaction  │  │    Risk      │ │
│  │  Manager     │  │   Service    │  │  Detection   │ │
│  │              │  │              │  │              │ │
│  │ - Create     │  │ - Sign       │  │ - Rules      │ │
│  │ - Delete     │  │ - Broadcast  │  │ - Analysis   │ │
│  │ - Restore    │  │ - Track      │  │ - Alerts     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │    Auth      │  │   Bridge     │  │   Storage    │ │
│  │  Service     │  │   Service    │  │   Service    │ │
│  │              │  │              │  │              │ │
│  │ - Login      │  │ - Transfer   │  │ - Database   │ │
│  │ - Sessions   │  │ - Track      │  │ - Cache      │ │
│  │ - Tokens     │  │ - Status     │  │ - Backup     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└────────────────┬────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────┐
│              Security Layer                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │  - AES-256-GCM Encryption                        │  │
│  │  - PBKDF2 Key Derivation (100k+ iterations)      │  │
│  │  - bcrypt Password Hashing                       │  │
│  │  - Zeroize Memory Protection                     │  │
│  │  - Input Validation & Sanitization               │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────┬────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────┐
│            Blockchain Layer                              │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ Ethereum │  │ Polygon  │  │   BSC    │  │Bitcoin │ │
│  │  Client  │  │  Client  │  │  Client  │  │ Client │ │
│  │          │  │          │  │          │  │        │ │
│  │ ethers-  │  │ ethers-  │  │ ethers-  │  │ Custom │ │
│  │   rs     │  │   rs     │  │   rs     │  │ Impl   │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └───┬────┘ │
│       │             │              │             │      │
│       └─────────────┴──────────────┴─────────────┘      │
│                   RPC Providers                          │
│         (Infura, Alchemy, QuickNode, etc.)              │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│              Data Persistence Layer                      │
│  ┌──────────────────────────────────────────────────┐  │
│  │  SQLite Database (SQLx)                          │  │
│  │  - Users           - Wallets                     │  │
│  │  - Transactions    - Sessions                    │  │
│  │  - Audit Logs      - Bridge History              │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 🗂️ Module Structure

### Backend (Rust)

```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library root
│
├── api/                    # API Layer (REST + WebSocket)
│   ├── server.rs          # Axum server setup
│   ├── auth.rs            # Authentication endpoints
│   ├── wallets.rs         # Wallet management endpoints
│   ├── transactions.rs    # Transaction endpoints
│   ├── anomaly_detection.rs # Risk detection endpoints
│   ├── bridge.rs          # Cross-chain bridge endpoints
│   │
│   ├── handlers/          # Request handlers
│   │   ├── wallet.rs      # Wallet operations
│   │   ├── transaction.rs # Transaction operations
│   │   ├── auth.rs        # Auth operations
│   │   └── ...
│   │
│   └── middleware/        # API middleware
│       ├── auth.rs        # JWT validation
│       ├── rate_limit.rs  # Rate limiting
│       ├── cors.rs        # CORS handling
│       └── error.rs       # Error handling
│
├── core/                   # Business Logic Layer
│   ├── wallet_manager/    # Wallet management (17 sub-modules)
│   │   ├── lifecycle.rs   # Create/delete/restore
│   │   ├── transactions.rs # Send/receive
│   │   ├── balance.rs     # Balance queries
│   │   ├── bridge.rs      # Cross-chain
│   │   ├── backup.rs      # Backup/export
│   │   ├── nonce.rs       # Nonce management
│   │   └── ...
│   │
│   ├── config.rs          # Configuration management
│   └── errors.rs          # Error definitions
│
├── blockchain/             # Blockchain Layer
│   ├── ethereum.rs        # Ethereum client
│   ├── bitcoin/           # Bitcoin client
│   │   ├── client.rs      # Bitcoin RPC client
│   │   ├── utxo.rs        # UTXO management
│   │   └── taproot.rs     # Taproot support
│   │
│   ├── bridge/            # Cross-chain bridge
│   │   ├── mod.rs         # Bridge logic
│   │   ├── ethereum_polygon.rs
│   │   └── ...
│   │
│   └── traits.rs          # Blockchain trait definitions
│
├── security/               # Security Layer
│   ├── encryption.rs      # AES-256-GCM implementation
│   ├── password_validator.rs # Password validation
│   ├── env_validator.rs   # Environment validation
│   ├── memory_protection.rs # Memory zeroization
│   └── secret.rs          # SecretVec wrapper
│
├── auth/                   # Authentication & Authorization
│   ├── service.rs         # Auth service
│   ├── session_manager.rs # Session management
│   ├── lockout.rs         # Account lockout
│   └── types.rs           # Auth types
│
├── anomaly_detection/      # Risk Detection System
│   ├── detector.rs        # Detection engine
│   ├── features.rs        # Feature extraction
│   ├── rules.rs           # Rule-based detection
│   ├── events.rs          # Event handling
│   └── storage.rs         # Detection history
│
├── storage/                # Data Persistence
│   ├── mod.rs             # Database abstraction
│   ├── models.rs          # Database models
│   └── migrations/        # SQL migrations
│
├── crypto/                 # Cryptographic Operations
│   ├── encryption.rs      # Symmetric encryption
│   ├── kdf.rs             # Key derivation
│   ├── signing/           # Digital signatures
│   └── quantum.rs         # Quantum-safe (experimental)
│
└── utils/                  # Utility Functions
    ├── validation.rs      # Input validation
    ├── logging.rs         # Logging utilities
    └── ...
```

---

### Frontend (TypeScript/React)

```
src/
├── App.tsx                # Main application component
├── index.tsx              # Application entry point
│
├── pages/                 # Page components
│   ├── WalletPage/        # Wallet dashboard
│   ├── SendPage/          # Send transaction
│   ├── HistoryPage/       # Transaction history
│   ├── BridgePage/        # Cross-chain bridge
│   ├── SettingsPage/      # Settings panel
│   └── AuthPage/          # Login/signup
│
├── components/            # Reusable components (100+)
│   ├── Layout/            # Layout components
│   ├── WalletSelector/    # Wallet dropdown
│   ├── NetworkSwitcher/   # Network selector
│   ├── TransactionPreview/
│   └── ...
│
├── context/               # React Context providers
│   ├── WalletContext.tsx  # Wallet state
│   ├── AuthContext.tsx    # Auth state
│   └── HardwareContext.tsx # Hardware wallet state
│
├── hooks/                 # Custom React hooks
│   ├── useWallet.ts       # Wallet operations
│   ├── useTransactions.ts # Transaction operations
│   ├── useAnomalyEvents.ts # Risk detection events
│   └── ...
│
├── services/              # API services
│   ├── api.ts             # Main API client
│   ├── wallet.ts          # Wallet API
│   ├── transaction.ts     # Transaction API
│   ├── risk.ts            # Risk detection API
│   └── ...
│
├── types/                 # TypeScript types
│   ├── wallet.ts
│   ├── transaction.ts
│   └── ...
│
└── utils/                 # Utility functions
    ├── validation.ts
    ├── formatting.ts
    └── ...
```

---

## 🔄 Data Flow

### 1. Wallet Creation Flow

```
┌─────────┐    1. Create Wallet Request
│  User   │───────────────────────────┐
│ (React) │                           │
└─────────┘                           ▼
                            ┌──────────────────┐
                            │  API Gateway     │
                            │  POST /wallets   │
                            └────────┬─────────┘
                                     │ 2. JWT Validation
                                     │ 3. Rate Limit Check
                                     ▼
                            ┌──────────────────┐
                            │ Wallet Manager   │
                            │ lifecycle.rs     │
                            └────────┬─────────┘
                                     │ 4. Generate Mnemonic (BIP39)
                                     │    24 words
                                     ▼
                            ┌──────────────────┐
                            │ HD Key Derivation│
                            │ (BIP32/44)       │
                            └────────┬─────────┘
                                     │ 5. Derive Keys
                                     │    m/44'/60'/0'/0/0 (ETH)
                                     │    m/84'/0'/0'/0/0 (BTC)
                                     ▼
                            ┌──────────────────┐
                            │  Encryption      │
                            │  AES-256-GCM     │
                            └────────┬─────────┘
                                     │ 6. Encrypt Private Keys
                                     │    with user password
                                     ▼
                            ┌──────────────────┐
                            │   Database       │
                            │   (SQLite)       │
                            └────────┬─────────┘
                                     │ 7. Store Encrypted Data
                                     ▼
┌─────────┐    8. Return Mnemonic (one-time only)
│  User   │◄──────────────────────────┘
│         │
│ ⚠️ Save│    User must save mnemonic!
│ Mnemonic│    Lost = permanent loss
└─────────┘
```

---

### 2. Transaction Flow

```
┌─────────┐    1. Send Transaction
│  User   │───────────────────────────┐
│         │    {to, amount, password} │
└─────────┘                           ▼
                            ┌──────────────────┐
                            │  API Gateway     │
                            │  POST /send      │
                            └────────┬─────────┘
                                     │ 2. Validate Input
                                     │    - Address format
                                     │    - Amount > 0
                                     │    - Password provided
                                     ▼
                            ┌──────────────────┐
                            │ Risk Detection   │
                            │  detector.rs     │
                            └────────┬─────────┘
                                     │ 3. Check Rules
                                     │    - Blacklist
                                     │    - High value
                                     │    - Dust attack
                                     ▼
                            ┌──────────────────┐
                            │ Decision Point   │
                            │ Block or Allow?  │
                            └────────┬─────────┘
                                     │
                        ┌────────────┴────────────┐
                        │ High Risk               │ Low Risk
                        ▼                         ▼
              ┌──────────────────┐      ┌──────────────────┐
              │ Return Warning   │      │ Transaction Svc  │
              │ Block if Critical│      │ transactions.rs  │
              └──────────────────┘      └────────┬─────────┘
                                                 │ 4. Decrypt Key
                                                 │    with password
                                                 ▼
                                        ┌──────────────────┐
                                        │ Blockchain Client│
                                        │ ethereum.rs      │
                                        └────────┬─────────┘
                                                 │ 5. Sign Transaction
                                                 │    ECDSA (secp256k1)
                                                 ▼
                                        ┌──────────────────┐
                                        │ RPC Provider     │
                                        │ Infura/Alchemy   │
                                        └────────┬─────────┘
                                                 │ 6. Broadcast
                                                 ▼
                                        ┌──────────────────┐
                                        │  Blockchain      │
                                        │  Network         │
                                        └────────┬─────────┘
                                                 │ 7. Confirmation
┌─────────┐    8. Return tx_hash                ▼
│  User   │◄──────────────────────────┘
│         │    Monitor tx status
└─────────┘
```

---

### 3. Authentication Flow

```
┌─────────┐    1. Login Request
│  User   │───────────────────────────┐
│         │    {username, password}   │
└─────────┘                           ▼
                            ┌──────────────────┐
                            │  API Gateway     │
                            │  POST /login     │
                            └────────┬─────────┘
                                     │ 2. Rate Limit Check
                                     │    (10 attempts/15min)
                                     ▼
                            ┌──────────────────┐
                            │  Auth Service    │
                            │  service.rs      │
                            └────────┬─────────┘
                                     │ 3. Query Database
                                     ▼
                            ┌──────────────────┐
                            │  Database        │
                            │  users table     │
                            └────────┬─────────┘
                                     │ 4. Get user record
                                     ▼
                            ┌──────────────────┐
                            │ Password Check   │
                            │ bcrypt::verify   │
                            └────────┬─────────┘
                                     │
                        ┌────────────┴────────────┐
                        │ Invalid                 │ Valid
                        ▼                         ▼
              ┌──────────────────┐      ┌──────────────────┐
              │ Increment Fails  │      │ Reset Fail Count │
              │ Check Lockout    │      │ Generate Tokens  │
              └────────┬─────────┘      └────────┬─────────┘
                       │                         │
                       │                         │ 5. Create JWT
                       │                         │    Access: 15min
                       │                         │    Refresh: 7 days
                       │                         ▼
                       │                ┌──────────────────┐
                       │                │ Session Manager  │
                       │                │ Create Session   │
                       │                └────────┬─────────┘
                       │                         │ 6. Store Session
                       ▼                         ▼
              ┌──────────────────┐      ┌──────────────────┐
              │ Return Error     │      │ Return Tokens    │
              │ 401 Unauthorized │      │ 200 OK           │
              └──────────────────┘      └────────┬─────────┘
                                                 │
┌─────────┐    7. Store Tokens                  │
│  User   │◄───────────────────────────────────┘
│         │    localStorage
│ Access  │    - access_token
│ Granted │    - refresh_token
└─────────┘
```

---

## 🔑 Design Patterns

### 1. Repository Pattern

**Purpose**: Abstract data access logic

```rust
// Trait definition
pub trait WalletRepository {
    async fn create(&self, wallet: Wallet) -> Result<()>;
    async fn get(&self, name: &str) -> Result<Option<Wallet>>;
    async fn delete(&self, name: &str) -> Result<()>;
}

// SQLite implementation
pub struct SqliteWalletRepository {
    pool: Pool<Sqlite>,
}

impl WalletRepository for SqliteWalletRepository {
    async fn create(&self, wallet: Wallet) -> Result<()> {
        sqlx::query!(
            "INSERT INTO wallets (name, encrypted_data) VALUES (?, ?)",
            wallet.name,
            wallet.encrypted_data
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

---

### 2. Strategy Pattern (Multi-chain)

**Purpose**: Swap blockchain implementations

```rust
// Common trait for all blockchain clients
#[async_trait]
pub trait BlockchainClient {
    async fn get_balance(&self, address: &str) -> Result<String>;
    async fn send_transaction(&self, tx: Transaction) -> Result<TxHash>;
    async fn get_transaction_history(&self, address: &str) -> Result<Vec<Tx>>;
}

// Ethereum implementation
pub struct EthereumClient {
    provider: Provider<Http>,
}

#[async_trait]
impl BlockchainClient for EthereumClient {
    async fn get_balance(&self, address: &str) -> Result<String> {
        // Ethereum-specific logic
    }
}

// Bitcoin implementation
pub struct BitcoinClient {
    // Different structure
}

#[async_trait]
impl BlockchainClient for BitcoinClient {
    async fn get_balance(&self, address: &str) -> Result<String> {
        // Bitcoin-specific logic (UTXO model)
    }
}

// Usage
let client: Box<dyn BlockchainClient> = match network {
    "ethereum" => Box::new(EthereumClient::new()),
    "bitcoin" => Box::new(BitcoinClient::new()),
    _ => unreachable!(),
};
```

---

### 3. Middleware Pattern

**Purpose**: Chain request processing

```rust
// Axum middleware composition
let app = Router::new()
    .route("/api/wallets", post(create_wallet))
    .layer(middleware::from_fn(jwt_auth_middleware))
    .layer(middleware::from_fn(rate_limit_middleware))
    .layer(middleware::from_fn(logging_middleware))
    .layer(CorsLayer::new()...);

// Execution order (reverse of declaration):
// Request → CORS → Logging → Rate Limit → JWT → Handler
```

---

### 4. Dependency Injection

**Purpose**: Testability and flexibility

```rust
// Service with dependencies
pub struct WalletService {
    repository: Arc<dyn WalletRepository>,
    blockchain: Arc<dyn BlockchainClient>,
    encryptor: Arc<Encryptor>,
}

impl WalletService {
    pub fn new(
        repository: Arc<dyn WalletRepository>,
        blockchain: Arc<dyn BlockchainClient>,
        encryptor: Arc<Encryptor>,
    ) -> Self {
        Self { repository, blockchain, encryptor }
    }
    
    pub async fn create_wallet(&self, name: &str) -> Result<Wallet> {
        // Use injected dependencies
    }
}

// Easy to mock in tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_wallet_service() {
        let mock_repo = Arc::new(MockRepository::new());
        let mock_blockchain = Arc::new(MockBlockchain::new());
        let mock_encryptor = Arc::new(MockEncryptor::new());
        
        let service = WalletService::new(mock_repo, mock_blockchain, mock_encryptor);
        // Test with mocks
    }
}
```

---

### 5. Event Bus Pattern

**Purpose**: Decouple components

```typescript
// Frontend event bus
export class EventBus {
    private listeners: Map<string, Function[]> = new Map();
    
    emit(event: string, data: any) {
        const handlers = this.listeners.get(event) || [];
        handlers.forEach(handler => handler(data));
    }
    
    on(event: string, handler: Function) {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, []);
        }
        this.listeners.get(event)!.push(handler);
    }
}

// Usage
eventBus.on('api-error', (error) => {
    showNotification(error.message);
});

// Somewhere else
eventBus.emit('api-error', { message: 'Network error' });
```

---

## 🔐 Security Architecture

### Defense in Depth

```
Layer 1: Network Security
├── HTTPS/TLS 1.3
├── Firewall rules
└── DDoS protection

Layer 2: Application Security
├── Input validation
├── CSRF protection
├── XSS prevention
└── Rate limiting

Layer 3: Authentication
├── JWT with refresh tokens
├── bcrypt password hashing
├── Account lockout
└── Session management

Layer 4: Authorization
├── Role-based access
├── Resource ownership
└── API key validation

Layer 5: Data Security
├── AES-256-GCM encryption
├── PBKDF2 key derivation
├── Secure random generation
└── Memory zeroization

Layer 6: Monitoring
├── Security event logging
├── Anomaly detection
├── Audit trails
└── Alert system
```

---

## 📊 Scalability Considerations

### Current Architecture (Single Node)

```
Limitations:
- Single database instance (SQLite)
- No horizontal scaling
- In-process rate limiting

Suitable for:
- Development
- Small deployments (< 1000 users)
- Personal/team use
```

---

### Future Scalability (Production)

```
Planned Improvements:

Database:
├── Migrate to PostgreSQL
├── Connection pooling (already implemented)
├── Read replicas
└── Database sharding (by user_id)

Caching:
├── Redis for session storage
├── Cache hot data (balances, prices)
└── Distributed cache

Load Balancing:
├── Multiple API instances
├── Nginx/HAProxy load balancer
└── Health check endpoints

Rate Limiting:
├── Redis-based distributed limiter
└── Per-user quotas

Async Processing:
├── Background job queue (tokio tasks)
├── Transaction status polling
└── Email notifications
```

---

## 🧪 Testing Strategy

### Test Pyramid

```
           ┌──────────┐
           │  E2E     │  10%  - Full system tests
           │  Tests   │       - Browser automation
          └──────────┘
         ┌──────────────┐
         │ Integration  │  30%  - API tests
         │   Tests      │       - Database tests
        └──────────────┘       - Multi-component
       ┌────────────────────┐
       │    Unit Tests      │  60%  - Function tests
       │                    │       - Logic tests
      └────────────────────┘       - Fast & isolated
```

---

## 🔄 CI/CD Pipeline

```
┌─────────────┐
│  Git Push   │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│ GitHub Actions  │
└──────┬──────────┘
       │
       ├─► Lint (cargo clippy)
       ├─► Format (cargo fmt)
       ├─► Security (cargo audit)
       ├─► Test (cargo test)
       ├─► Coverage (tarpaulin)
       └─► Build (cargo build)
       │
       ▼
┌─────────────────┐
│ Quality Gates   │
│ - Coverage > 80%│
│ - No warnings   │
│ - All tests pass│
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│   Merge PR      │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│ Deploy (Manual) │
└─────────────────┘
```

---

## 📚 Technology Choices

### Backend: Why Rust?

```
✅ Memory Safety (no garbage collector)
✅ Performance (comparable to C/C++)
✅ Fearless Concurrency (async/await)
✅ Strong Type System
✅ Excellent Tooling (cargo, clippy)
✅ Growing Blockchain Ecosystem
✅ Security (ownership prevents many bugs)
```

---

### Frontend: Why React + TypeScript?

```
✅ Large Ecosystem
✅ Component Reusability
✅ Type Safety (TypeScript)
✅ Developer Experience
✅ Community Support
✅ Web3 Integration (ethers.js)
```

---

### Database: Why SQLite?

```
✅ Embedded (no server)
✅ Zero Configuration
✅ Reliable (ACID)
✅ Fast for small datasets
✅ Easy backup (single file)

Migration Path:
- Development: SQLite
- Production: PostgreSQL (same API via SQLx)
```

---

## 🎯 Design Decisions

### 1. Non-Custodial Architecture

**Decision**: Users control private keys  
**Rationale**: Maximum security and user sovereignty  
**Tradeoff**: No "forgot password" recovery  

---

### 2. Rule-Based Risk Detection

**Decision**: Rules instead of ML (initially)  
**Rationale**: Explainable, debuggable, fast  
**Future**: Hybrid model (rules + ML)  

---

### 3. Synchronous API

**Decision**: Request-response pattern  
**Rationale**: Simpler client code  
**Tradeoff**: Long transactions block client  
**Mitigation**: WebSocket for real-time updates  

---

### 4. Monolithic Backend

**Decision**: Single binary  
**Rationale**: Simpler deployment, suitable for scale  
**Future**: Microservices if needed  

---

## 📖 Further Reading

- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Axum Documentation](https://docs.rs/axum/)
- [ethers-rs Docs](https://docs.rs/ethers/)
- [React Documentation](https://react.dev/)
- [BIP32 Specification](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki)
- [EIP-1559 Specification](https://eips.ethereum.org/EIPS/eip-1559)

---

**Architecture Version**: 1.0  
**Last Updated**: November 6, 2025  
**Status**: Production Ready  
**Maintainer**: @DarkCrab-Rust



## Latest Updates (November 2025)

- Complete English internationalization (2,143 lines of comments)
- Major security improvements (A+ rating maintained)
- Project cleanup (56 files removed)
- Documentation enhancement
- All commits now in English for international collaboration

