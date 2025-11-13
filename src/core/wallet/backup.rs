// filepath: src/core/wallet/backup.rs
use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use zeroize::Zeroizing;

use crate::core::errors::WalletError;
use crate::storage::WalletStorageTrait;
/// Backs up a wallet by generating a new mnemonic and returning it as a zeroizing buffer.
pub async fn backup_wallet(
    _storage: &Arc<dyn WalletStorageTrait + Send + Sync>,
    wallet_name: &str,
) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    info!("Backing up wallet: {}", wallet_name);
    // Generate mnemonic as backup (canonical create::generate_mnemonic returns SecretVec)
    let mnemonic_z = crate::core::wallet::create::generate_mnemonic()
        .map_err(|e| WalletError::MnemonicError(e.to_string()))?;
    Ok(mnemonic_z)
}
