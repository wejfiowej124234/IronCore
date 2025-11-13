## Copilot instructions for DeFi Hot Wallet (Rust)

This brief guide makes AI agents productive here. Be concrete, follow repo patterns, and cite files/commands from this codebase.

### Big picture
- Layered Rust app: API/CLI → Core services → Security/Crypto → Chain clients → Storage. See `src/lib.rs` exports; entry points in `src/main.rs` (bin: `hot_wallet`) and `src/bin/wallet-cli.rs` (bin: `wallet-cli`).
- Orchestrator: `src/core/wallet_manager.rs` (`WalletManager::new(&WalletConfig).await?`). Chains wired via `blockchain::traits::BlockchainClient`; bridges via `blockchain::bridge` (mockable).
- ETH integration uses `ethers` (abigen+rustls). Other EVM chains (Polygon, BSC) use the same client.

### Where to look
- API server: `src/api/server.rs` (routes, auth, Router), handlers in `src/api/handlers.rs`, types in `src/api/types.rs`.
- Core/config/errors: `src/core/{wallet_manager.rs,config.rs,errors.rs}`. Storage in `src/storage/` (SQLx), metrics in `src/api/handlers.rs::metrics_handler`.
- Features/deps: see `Cargo.toml` (`strict_security`, `sop_patch_tests`, `test-env`, `[patch.crates-io] elliptic-curve-tools`).

### Conventions that matter
- Async-first with `tokio`. Logging via `tracing` + `init_logging()` in `src/main.rs`.
- Errors: server/main often use `anyhow::Result`, internal logic returns `core::errors::WalletError` (map errors accordingly).
- Secrets: use `zeroize` types; avoid logging secrets. Sensitive flows go through `SecureWalletData` and are zeroized.
- Auth: API requires header `Authorization: <API_KEY>` when `API_KEY` env is set. No "Bearer" prefix parsing.

### Running and calling the API
- Server reads `DATABASE_URL` or falls back to `sqlite://./wallets.db`.
- Start: `cargo run --bin hot_wallet -- server` (defaults to 127.0.0.1:8080).
- Health/metrics: GET `/api/health`, `/api/metrics`.
- Wallets: POST `/api/wallets` {name, quantum_safe}, GET `/api/wallets`, DELETE `/api/wallets/:name`.
- Send/balance: GET `/api/wallets/:name/balance?network=eth`, POST `/api/wallets/:name/send`.
- Bridge: POST `/api/bridge`. If `BRIDGE_MOCK_FORCE_SUCCESS=1`, returns `{"bridge_tx_id":"mock_bridge_tx_hash"}` without real signing.

### Tests and useful toggles
- Tests live in `src/api/*_tests.rs`, `tests/`, using `tokio::test`, `serial_test`, and `axum-test`. Avoid live RPCs; use the provided mocks.
- Test helpers: `WalletServer::new_for_test(...)` injects a deterministic master key via `core::wallet_manager::set_test_master_key_default` for decrypt/sign flows.
- Optional: feature `test-env` auto-sets `BRIDGE_MOCK_FORCE_SUCCESS=1` in `src/test_env.rs` for stable bridge tests.

### Integration points
- Blockchain RPC clients in `src/blockchain/` (`EthereumClient`). Add networks in `core::config::WalletConfig` and ensure `blockchain_clients` wiring in `WalletManager::new`.
- Storage uses SQLx (sqlite/postgres features enabled). Default URLs use sqlite.
- Local patch: `patches/elliptic-curve-tools` via `[patch.crates-io]`; prefer this crate shape when adding curve helpers.

### Gotchas specific to this repo
- Use `DATABASE_URL` (not `WALLET_DATABASE_URL`). SQLite examples: `sqlite://./wallets.db`.
- Auth header must equal the API key exactly; do not prepend "Bearer ".
- Unsupported chains return 404 "Unsupported chain" at the bridge endpoint; ensure networks exist in `WalletConfig.blockchain.networks`.
- Windows: target-specific deps (`windows`, optional `winapi`) are guarded; keep cross-platform `cfg` usage.

### Minimal PR checklist
1) `cargo fmt` and `cargo clippy -- -D warnings`. 2) Run targeted tests (`cargo test <name>`). 3) Don’t log secrets; zeroize sensitive buffers. 4) Update README/docs when changing public flags or endpoints.

If any area needs deeper examples (DB layer, WalletManager flows, signing/bridge mocks), ask and reference the exact files above.

### CI notes (important)
- Tests in CI run with the `test-env` feature so `src/test_env.rs` runs a small ctor initializer that sets:
	- `WALLET_ENC_KEY` (base64 of 32 zero bytes), `TEST_SKIP_DECRYPT=1`, `BRIDGE_MOCK_FORCE_SUCCESS=1`, `BRIDGE_MOCK=1`.
- Bridge tests rely on these env vars for deterministic results. If you run locally without the feature, set them yourself or pass `--features test-env`.
- Super Linter only validates YAML and ignores generated/third-party paths: `vendor/`, `target/`, `defi-target/`, `.vscode/`, `patches/`.
- Merge-conflict checks are PR-only and non-blocking; they configure a bot Git identity to avoid false failures. Actual conflicts are still reported in PRs.
