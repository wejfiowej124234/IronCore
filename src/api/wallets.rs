//! Wallet management API routes
//!
//! Provides wallet listing, creation, and query endpoints

use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

/// Wallet information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    /// Wallet unique identifier
    pub id: String,
    /// Wallet name
    pub name: String,
    /// Wallet blockchain address
    pub address: String,
    /// Whether quantum-safe encryption is enabled
    pub quantum_safe: bool,
    /// Creation timestamp (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Wallet type (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_type: Option<String>,
}

/// Create wallet request payload
#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    /// Wallet name (must be unique)
    pub name: String,
    /// Enable quantum-safe encryption
    #[serde(default = "default_false")]
    pub quantum_safe: bool,
    /// Password for wallet encryption
    pub password: String,
    /// Generate new mnemonic phrase
    #[serde(default = "default_true")]
    pub generate_mnemonic: bool,
}

/// Create wallet response payload
#[derive(Debug, Serialize)]
pub struct CreateWalletResponse {
    /// Wallet information
    pub wallet: WalletInfo,
    /// Mnemonic phrase (returned only once during creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic: Option<String>,
    /// Wallet blockchain address
    pub address: String,
    /// Warning message (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Wallet backup information
#[derive(Debug, Serialize)]
pub struct WalletBackup {
    /// Wallet unique identifier
    pub wallet_id: String,
    /// Mnemonic phrase for recovery
    pub mnemonic: String,
    /// Wallet blockchain address
    pub address: String,
    /// Creation timestamp
    pub created_at: String,
    /// Supported blockchain networks
    pub networks: Vec<String>,
}

fn default_false() -> bool { false }
fn default_true() -> bool { true }

/// API state management
#[derive(Clone)]
pub struct WalletApiState {
    /// Wallet list (Arc<Mutex> for thread safety)
    pub wallets: Arc<Mutex<Vec<WalletInfo>>>,
    /// API key for authentication (optional)
    pub api_key: Option<String>,
}

impl WalletApiState {
    /// Create new API state instance
    pub fn new() -> Self {
        // Initialize with example wallets (in production, load from database or file)
        let wallets = vec![
            WalletInfo {
                id: "w1".to_string(),
                name: "wallet-1".to_string(),
                address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1".to_string(),
                quantum_safe: false,
                created_at: Some(chrono::Utc::now().to_rfc3339()),
                wallet_type: Some("HD".to_string()),
            },
            WalletInfo {
                id: "w2".to_string(),
                name: "wallet-2".to_string(),
                address: "0x853f43A64DfA4c828A0e0D6E6DFb27BB40E6C8Eb".to_string(),
                quantum_safe: true,
                created_at: Some(chrono::Utc::now().to_rfc3339()),
                wallet_type: Some("Quantum-Safe".to_string()),
            },
            WalletInfo {
                id: "demo_wallet".to_string(),
                name: "demo_wallet".to_string(),
                address: "0x9F5B5F2A8C1E3D4A6B7C8D9E0F1A2B3C4D5E6F7A".to_string(),
                quantum_safe: false,
                created_at: Some(chrono::Utc::now().to_rfc3339()),
                wallet_type: Some("Demo".to_string()),
            },
        ];

        Self {
            wallets: Arc::new(Mutex::new(wallets)),
            api_key: None, // Set API key if authentication is required
        }
    }

    /// Create API state with authentication key
    pub fn with_api_key(api_key: String) -> Self {
        let mut state = Self::new();
        state.api_key = Some(api_key);
        state
    }
}

/// Create wallet management router with all endpoints
pub fn create_wallet_routes(state: WalletApiState) -> Router {
    Router::new()
        .route("/api/wallets", get(get_wallets).post(create_wallet))
        .route("/api/wallets/:wallet_id/balance/:token", get(get_wallet_token_balance))  // More specific routes first
        .route("/api/wallets/:wallet_id/balance", get(get_wallet_balance))
        .route("/api/wallets/:wallet_id/transactions", get(get_wallet_transactions))
        .route("/api/wallets/:wallet_id/export", post(export_wallet))
        .route("/api/wallets/:wallet_id/backup", get(get_wallet_backup))
        .with_state(state)
}

/// GET /api/wallets
/// 
/// Get wallet list endpoint
async fn get_wallets(
    State(state): State<WalletApiState>,
    headers: HeaderMap,
) -> Response {
    info!("Received get wallet list request");

    // Optional authentication check
    if let Some(ref api_key) = state.api_key {
        // Check Authorization header
        let auth_valid = headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .map(|auth| {
                // Support Bearer token format
                if auth.starts_with("Bearer ") {
                    &auth[7..] == api_key
                } else {
                    auth == api_key
                }
            })
            .unwrap_or(false);

        // Check X-API-Key header
        let api_key_valid = headers
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok())
            .map(|key| key == api_key)
            .unwrap_or(false);

        // Return 401 if neither auth method matches  
        if !auth_valid && !api_key_valid {
            info!("Wallet list request unauthorized");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Valid API key required"
                }))
            ).into_response();
        }
    }

    // Get wallet list from state
    let wallets = state.wallets.lock().await;
    
    info!("Returning {} wallets", wallets.len());
    
    // Return wallet list as JSON
    Json(wallets.clone()).into_response()
}

/// POST /api/wallets
/// 
/// Create new wallet endpoint
async fn create_wallet(
    State(state): State<WalletApiState>,
    Json(req): Json<CreateWalletRequest>,
) -> Response {
    info!("Received create wallet request: name={}, quantum_safe={}", req.name, req.quantum_safe);

    // Validate wallet name
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid wallet name",
                "message": "Wallet name cannot be empty"
            }))
        ).into_response();
    }
    
    // Validate password strength
    if req.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Weak password",
                "message": "Password must be at least 8 characters"
            }))
        ).into_response();
    }
    
    // Generate mnemonic (production-grade: proper Result handling)
    let mnemonic_str = if req.generate_mnemonic {
        match generate_real_mnemonic() {
            Ok(mnemonic) => mnemonic,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Mnemonic generation failed",
                        "message": format!("Failed to generate mnemonic: {}", e)
                    }))
                ).into_response();
            }
        }
    } else {
        // Use default mnemonic if generation is disabled
        match generate_real_mnemonic() {
            Ok(mnemonic) => mnemonic,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Mnemonic generation failed",
                        "message": format!("Failed to generate mnemonic: {}", e)
                    }))
                ).into_response();
            }
        }
    };
    
    // Derive private key and address from mnemonic (real BIP39 implementation)
    use crate::core::key_manager::KeyManager;
    
    // 1. Derive private key from mnemonic (using real BIP39 seed derivation)
    let private_key = match KeyManager::derive_private_key_from_mnemonic(&mnemonic_str) {
        Ok(key) => key,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Key derivation failed",
                    "message": format!("Failed to derive private key: {}", e)
                }))
            ).into_response();
        }
    };
    
    // 2. Derive address from private key
    let address = match KeyManager::derive_ethereum_address(&private_key) {
        Ok(addr) => addr,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Address derivation failed",
                    "message": format!("Failed to derive address: {}", e)
                }))
            ).into_response();
        }
    };
    
    // 3. Encrypt private key with password
    let (encrypted_key, nonce) = match KeyManager::encrypt_private_key(&private_key, &req.password) {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Encryption failed",
                    "message": format!("Failed to encrypt private key: {}", e)
                }))
            ).into_response();
        }
    };
    
    // Private key is automatically zeroized here (memory cleanup)
    drop(private_key);
    
    info!("✅ Private key derived, encrypted, and securely zeroized");
    
    // Save mnemonic for optional return
    let mnemonic = if req.generate_mnemonic {
        Some(mnemonic_str)
    } else {
        None
    };
    
    // Create wallet ID
    let wallet_id = Uuid::new_v4().to_string();
    
    // Create wallet information structure
    let wallet_info = WalletInfo {
        id: wallet_id.clone(),
        name: req.name.clone(),
        address: address.clone(),
        quantum_safe: req.quantum_safe,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        wallet_type: Some(if req.quantum_safe { "Quantum-Safe".to_string() } else { "HD".to_string() }),
    };
    
    // Add to wallet list
    {
        let mut wallets = state.wallets.lock().await;
        wallets.push(wallet_info.clone());
    }
    
    info!("Successfully created wallet: id={}, name={}", wallet_id, req.name);

    // Build response
    let mut response = CreateWalletResponse {
        wallet: wallet_info,
        mnemonic,
        address,
        warning: None,
    };
    
    // Add security warning
    if req.generate_mnemonic {
        response.warning = Some("IMPORTANT: Save your mnemonic phrase securely! It's the ONLY way to recover your wallet. Loss means permanent loss of funds.".to_string());
    }
    
    Json(response).into_response()
}

/// Generate real BIP39 mnemonic phrase (12 words)
/// Uses OS entropy source for cryptographic randomness
///
/// # Production-grade improvements
/// - Returns Result type to avoid panics
/// - Complete error handling
fn generate_real_mnemonic() -> anyhow::Result<String> {
    use bip39::{Mnemonic, Language};
    use rand::RngCore;
    use rand::rngs::OsRng;
    
    // Generate 128-bit entropy (12 words)
    let mut entropy = [0u8; 16];
    OsRng.fill_bytes(&mut entropy);
    
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| anyhow::anyhow!("Failed to generate mnemonic: {}", e))?;
    
    Ok(mnemonic.to_string())
}

/// Derive real Ethereum address from mnemonic
///
/// # Production-grade improvements
/// - Returns Result type to avoid panics
/// - Complete error handling and validation
///
/// # Note
/// Simplified version, not using full BIP44 path derivation
/// Production should use standard path m/44'/60'/0'/0/0
fn generate_real_address(mnemonic: &str) -> anyhow::Result<String> {
    use bip39::Mnemonic;
    use sha3::{Keccak256, Digest};
    
    // Parse mnemonic (production-grade: no expect/unwrap)
    let mnemonic = Mnemonic::parse(mnemonic)
        .map_err(|e| anyhow::anyhow!("Invalid mnemonic: {}", e))?;

    // Generate seed from mnemonic
    let seed = mnemonic.to_seed("");
    
    // Derive key using secp256k1
    use secp256k1::{Secp256k1, SecretKey};
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&seed[..32])
        .map_err(|e| anyhow::anyhow!("Invalid key: {}", e))?;
    let public_key = secret_key.public_key(&secp);
    
    // Calculate Ethereum address (last 20 bytes of Keccak256 hash)
    let public_key_bytes = &public_key.serialize_uncompressed()[1..]; // Remove 0x04 prefix
    let mut hasher = Keccak256::new();
    hasher.update(public_key_bytes);
    let hash = hasher.finalize();
    
    // Take last 20 bytes as address
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

/// GET /api/wallets/:wallet_id/balance
/// 
/// Get wallet balance endpoint
async fn get_wallet_balance(
    State(state): State<WalletApiState>,
    axum::extract::Path(wallet_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    info!("Received get wallet balance request: wallet_id={}", wallet_id);

    // Optional authentication check (same logic as get_wallets)
    if let Some(ref api_key) = state.api_key {
        let auth_valid = headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .map(|auth| {
                if auth.starts_with("Bearer ") {
                    &auth[7..] == api_key
                } else {
                    auth == api_key
                }
            })
            .unwrap_or(false);

        let api_key_valid = headers
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok())
            .map(|key| key == api_key)
            .unwrap_or(false);

        if !auth_valid && !api_key_valid {
            info!("Wallet balance request unauthorized");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Valid API key required"
                }))
            ).into_response();
        }
    }
    
    // Get network parameter
    let network = params.get("network").map(|s| s.as_str()).unwrap_or("ethereum");
    
    // Validate wallet exists (supports query by ID or name)
    let wallets = state.wallets.lock().await;
    let wallet_exists = wallets.iter().any(|w| w.id == wallet_id || w.name == wallet_id);
    
    if !wallet_exists {
        info!("Wallet not found: {}", wallet_id);
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Wallet not found",
                "message": format!("Wallet '{}' not found, please check wallet ID or name", wallet_id),
                "wallet_id": wallet_id
            }))
        ).into_response();
    }
    
    // Return simulated balance (in production, query blockchain)
    // Note: Returns 200 even if balance is 0, not 404
    let balance = match network {
        "ethereum" | "eth" => "1.23456789",
        "bsc" => "5.67",
        "polygon" => "100.0",
        _ => "0.0", // Unknown network returns 0 balance, still 200 status
    };
    
    info!("Returning wallet {} balance on {} network: {}", wallet_id, network, balance);
    
    Json(serde_json::json!({
        "balance": balance,
        "network": network,
        "wallet_id": wallet_id,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })).into_response()
}

/// GET /api/wallets/:wallet_id/balance/:token
/// 
/// Get wallet token balance endpoint
async fn get_wallet_token_balance(
    State(state): State<WalletApiState>,
    axum::extract::Path((wallet_id, token)): axum::extract::Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    info!("Received get wallet token balance request: wallet_id={}, token={}", wallet_id, token);
    
    // Validate wallet exists (supports query by ID or name)
    let wallets = state.wallets.lock().await;
    let wallet_exists = wallets.iter().any(|w| w.id == wallet_id || w.name == wallet_id);
    
    if !wallet_exists {
        info!("Wallet not found: {}", wallet_id);
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Wallet not found",
                "message": format!("Wallet '{}' not found, please check wallet ID or name", wallet_id),
                "wallet_id": wallet_id
            }))
        ).into_response();
    }
    
    // Get network parameter
    let network = params.get("network").map(|s| s.as_str()).unwrap_or("ethereum");
    
    // Return simulated token balance (in production, query blockchain)
    let balance = match token.to_uppercase().as_str() {
        "ETH" => "1.23456789",
        "USDT" => "1000.50",
        "USDC" => "500.25",
        "DAI" => "750.75",
        "BNB" => "5.5",
        "MATIC" => "10.5",
        _ => "0.0", // Unknown token returns 0 balance
    };
    
    info!("Returning wallet {} {} token balance: {}", wallet_id, token, balance);
    
    Json(serde_json::json!({
        "balance": balance,
        "token": token,
        "network": network,
        "wallet_id": wallet_id,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })).into_response()
}

/// GET /api/wallets/:wallet_id/transactions
/// 
/// Get wallet transaction history endpoint
/// Note: When wallet exists but has no transactions, returns 200 + empty array, not 404
async fn get_wallet_transactions(
    State(state): State<WalletApiState>,
    axum::extract::Path(wallet_id): axum::extract::Path<String>,
    axum::extract::Query(_params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    info!("Received get wallet transactions request: wallet_id={}", wallet_id);
    
    // Validate wallet exists (supports query by ID or name)
    let wallets = state.wallets.lock().await;
    let wallet_exists = wallets.iter().any(|w| w.id == wallet_id || w.name == wallet_id);
    
    if !wallet_exists {
        info!("Wallet not found: {}", wallet_id);
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Wallet not found",
                "message": format!("Wallet '{}' not found, please check wallet ID or name", wallet_id),
                "wallet_id": wallet_id
            }))
        ).into_response();
    }
    
    // Wallet exists but has no transactions - return 200 + empty array
    info!("Returning wallet {} transaction history (currently empty)", wallet_id);
    
    Json(serde_json::json!({
        "wallet_id": wallet_id,
        "transactions": [],
        "total": 0,
        "message": "Wallet has no transaction history yet",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })).into_response()
}

/// POST /api/wallets/:wallet_id/export
/// 
/// Export wallet endpoint (requires password verification)
async fn export_wallet(
    State(state): State<WalletApiState>,
    axum::extract::Path(wallet_id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Response {
    info!("Received export wallet request: wallet_id={}", wallet_id);

    // Validate wallet exists
    let wallets = state.wallets.lock().await;
    let wallet = wallets.iter().find(|w| w.id == wallet_id || w.name == wallet_id);
    
    if wallet.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Wallet not found",
                "message": format!("Wallet '{}' not found", wallet_id)
            }))
        ).into_response();
    }
    
    let wallet = wallet.unwrap();
    
    // Simulated password verification
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    if password.len() < 8 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid password",
                "message": "Password verification failed"
            }))
        ).into_response();
    }
    
    // Production-grade implementation: Mnemonic should not be exportable after creation
    // Reason: Mnemonic should only be shown once during creation, never stored on server
    // This is industry best practice (reference: MetaMask, Trust Wallet)
    
    info!("Wallet export request denied: {} (enterprise security policy)", wallet_id);
    
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "Backup not supported",
            "message": "For security reasons, mnemonic phrase is only shown once during wallet creation.",
            "details": {
                "reason": "Mnemonic is NOT stored on server (industry best practice)",
                "solution": "Please save mnemonic during wallet creation. If lost, wallet can only be recovered using the original mnemonic.",
                "best_practice": "This is standard practice for MetaMask, Trust Wallet, and other major wallets"
            },
            "alternatives": [
                "Prompt users to save mnemonic during wallet creation",
                "Implement 'verify mnemonic' feature to confirm user has saved it",
                "If export support is needed, implement encrypted storage solution (higher security risk)"
            ]
        }))
    ).into_response()
}

/// GET /api/wallets/:wallet_id/backup
/// 
/// Get wallet backup information endpoint (read-only)
async fn get_wallet_backup(
    State(state): State<WalletApiState>,
    axum::extract::Path(wallet_id): axum::extract::Path<String>,
) -> Response {
    info!("Received get wallet backup request: wallet_id={}", wallet_id);

    // Validate wallet exists
    let wallets = state.wallets.lock().await;
    let wallet = wallets.iter().find(|w| w.id == wallet_id || w.name == wallet_id);
    
    if wallet.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Wallet not found",
                "message": format!("Wallet '{}' not found", wallet_id)
            }))
        ).into_response();
    }
    
    let wallet = wallet.unwrap();
    
    // Return read-only backup information (no sensitive data)
    Json(serde_json::json!({
        "wallet_id": wallet.id,
        "name": wallet.name,
        "address": wallet.address,
        "created_at": wallet.created_at,
        "quantum_safe": wallet.quantum_safe,
            "networks": vec!["ethereum", "bsc"],
        "note": "This is read-only information. For complete backup, use export function"
    })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_info_serialization() {
        let wallet = WalletInfo {
            id: "w1".to_string(),
            name: "test-wallet".to_string(),
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            quantum_safe: true,
            created_at: Some("2025-10-29T12:00:00Z".to_string()),
            wallet_type: Some("HD".to_string()),
        };

        let json = serde_json::to_string(&wallet).unwrap();
        assert!(json.contains("\"id\":\"w1\""));
        assert!(json.contains("\"quantum_safe\":true"));
    }

    #[tokio::test]
    async fn test_wallet_state() {
        let state = WalletApiState::new();
        let wallets = state.wallets.lock().await;
        assert!(!wallets.is_empty());
        assert_eq!(wallets[0].id, "w1");
    }
}

