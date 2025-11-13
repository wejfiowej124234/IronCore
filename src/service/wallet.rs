use crate::core::domain::Tx;
use crate::mvp::Wallet;
use anyhow::Result;

/// Wallet service layer.
#[derive(Debug, Default)]
pub struct WalletService;

impl WalletService {
    /// Creates a new `WalletService`.
    pub fn new() -> Self {
        Self
    }

    /// Creates a new wallet from a mnemonic phrase.
    pub async fn create_wallet(&self, mnemonic: &str) -> Result<Wallet> {
        Wallet::from_mnemonic(mnemonic)
    }

    /// Sends a transaction from a wallet.
    pub async fn send_tx(&self, wallet: &Wallet, to: &str, amount: u64) -> Result<Tx> {
        // In a real implementation, this would involve:
        // 1. Getting the private key for the wallet.
        // 2. Creating and signing the transaction.
        // 3. Sending it to the network.
        // For now, we'll just create a mock Tx.
        let _ = wallet; // a real implementation would use the wallet
        let tx = Tx::new(wallet, to, amount);
        Ok(tx)
    }
}
