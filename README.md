# Blockchain Wallet Backend

A REST API server for cryptocurrency wallet management built with Rust.

## Overview

This is a backend service that provides wallet creation, transaction management, and multi-chain blockchain integration through a RESTful API.

## Features

- Multi-chain support (Ethereum, Bitcoin, Polygon, BSC)
- Wallet creation and management with BIP39/BIP44
- Transaction sending and tracking
- Balance queries across multiple chains
- User authentication with JWT
- Cross-chain bridge and swap integration
- NFT and GameFi asset management

## Technology Stack

- **Language**: Rust
- **Framework**: Axum (async web framework)
- **Runtime**: Tokio (async runtime)
- **Database**: SQLite with SQLx
- **Cryptography**: RustCrypto ecosystem
- **Authentication**: JWT with Argon2id password hashing

## Architecture

```
┌─────────────────┐
│   REST API      │  (Axum + Tokio)
├─────────────────┤
│  Core Services  │  (Wallet, Transaction, Auth)
├─────────────────┤
│   Database      │  (SQLite + SQLx)
└─────────────────┘
        ↓
   Blockchain
   Networks
```

## API Endpoints

### Authentication
- POST `/api/register` - User registration
- POST `/api/login` - User login
- POST `/api/logout` - User logout

### Wallet
- POST `/api/wallet/create` - Create new wallet
- GET `/api/wallet/list` - List user wallets
- GET `/api/wallet/balance` - Get wallet balance

### Transactions
- POST `/api/transaction/send` - Send transaction
- GET `/api/transaction/history` - Get transaction history
- GET `/api/transaction/status` - Check transaction status

### Advanced
- POST `/api/bridge/transfer` - Cross-chain bridge
- POST `/api/swap/execute` - Token swap
- GET `/api/nft/list` - List NFT assets

See [API_DOCUMENTATION.md](API_DOCUMENTATION.md) for complete API reference.

## Quick Start

### Prerequisites
- Rust 1.75+
- SQLite 3

### Installation

```bash
# Clone repository
git clone https://github.com/DarkCrab-Rust/Rust-Blockchain-Secure-Wallet.git
cd Rust-Blockchain-Secure-Wallet

# Copy environment configuration
cp .env.example .env

# Edit .env with your settings
# Set blockchain RPC endpoints, JWT secret, etc.

# Build and run
cargo build --release
cargo run --release
```

The server will start on `http://localhost:8888`.

### Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html
```

## Configuration

Key environment variables:

```env
DATABASE_URL=sqlite:users.db
JWT_SECRET=your-secret-key
ETHEREUM_RPC=https://eth-mainnet.alchemyapi.io/v2/YOUR-KEY
POLYGON_RPC=https://polygon-mainnet.alchemyapi.io/v2/YOUR-KEY
BSC_RPC=https://bsc-dataseed.binance.org/
```

## Project Structure

```
src/
├── api/            # API route handlers
├── core/           # Core business logic
│   ├── wallet/     # Wallet management
│   ├── crypto/     # Cryptography utilities
│   └── bip/        # BIP39/BIP44 implementation
├── services/       # External service integrations
├── db/             # Database operations
└── main.rs         # Application entry point
```

## Security

- Passwords hashed with Argon2id
- Private keys encrypted with AES-256-GCM
- JWT tokens with 1-hour expiration
- SQL injection prevention with parameterized queries
- Input validation on all endpoints
- CORS configuration for production

## Testing

- 348 unit and integration tests
- Test coverage: 85%+
- Includes stress tests for concurrent operations

## License

MIT License - see [LICENSE](LICENSE) for details

## Contributing

Contributions welcome. Please open an issue or pull request.

## Related Projects

- **Web Frontend**: [blockchain-wallet-ui](https://github.com/DarkCrab-Rust/blockchain-wallet-ui)
- **Mobile DApp**: [IronLink-DApp](https://github.com/DarkCrab-Rust/IronLink-DApp)
- **Web Wallet (Rust)**: [IronForge](https://github.com/DarkCrab-Rust/IronForge)

## Contact

GitHub Issues: https://github.com/DarkCrab-Rust/Rust-Blockchain-Secure-Wallet/issues
