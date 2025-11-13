//! KEK rotation integration tests

mod util;

#[cfg(feature = "test-env")]
use base64::Engine;
#[cfg(feature = "test-env")]
use defi_hot_wallet::core::config::WalletConfig;
#[cfg(feature = "test-env")]
use defi_hot_wallet::core::wallet_info::SecureWalletData;
#[cfg(feature = "test-env")]
use defi_hot_wallet::core::wallet_manager::WalletManager;
#[cfg(feature = "test-env")]
use defi_hot_wallet::storage::WalletStorage;
#[cfg(feature = "test-env")]
use std::sync::Arc;

#[cfg(feature = "test-env")]
fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(feature = "test-env")]
async fn setup_manager() -> (WalletManager, Arc<WalletStorage>) {
    let storage = Arc::new(
        WalletStorage::new_with_url("sqlite::memory:").await.expect("in-memory storage init"),
    );
    let cfg = WalletConfig::default();
    let wm = WalletManager::new_with_storage(cfg, storage.clone()).await.expect("wm init");
    (wm, storage)
}

#[tokio::test]
#[cfg(feature = "test-env")]
async fn rotate_kek_success_and_idempotent() {
    // Ensure deterministic test env (WALLET_ENC_KEY/test overrides)
    util::set_test_env();
    let (wm, storage) = setup_manager().await;

    // Create a wallet using the default KEK (no kek_id set yet)
    let name = "rot8_wallet";
    wm.create_wallet(name, false).await.expect("create wallet");
    // 验证钱包已创建
    let wallets = wm.list_wallets().await.expect("list wallets");
    assert!(wallets.iter().any(|w| w.name == name), "钱包应该已创建");

    // Provide a new KEK under WALLET_ENC_KEY_BLUE (32 bytes of 0x42)
    let new_kek = std::iter::repeat_n(0x42u8, 32).collect::<Vec<u8>>();
    std::env::set_var("WALLET_ENC_KEY_BLUE", b64(&new_kek));

    // Perform rotation to BLUE
    wm.rotate_envelope_kek_for_wallet(name, Some("BLUE")).await.expect("rotate to BLUE");

    // Verify: wallet persists with kek_id = Some("BLUE") and can be decrypted by calling backup
    let serialized = storage.load_wallet(name).await.expect("load wallet").0;
    let wd: SecureWalletData = bincode::deserialize(&serialized).expect("deserialize");
    assert_eq!(wd.kek_id.as_deref(), Some("BLUE"));

    // backup requires decrypt, which will use WALLET_ENC_KEY_BLUE internally
    let seed = wm.backup_wallet(name).await.expect("backup after rotate");
    assert!(!seed.is_empty());

    // Idempotent: rotate again to BLUE should be a no-op and succeed
    wm.rotate_envelope_kek_for_wallet(name, Some("BLUE")).await.expect("idempotent rotate");
}

#[tokio::test]
#[cfg(feature = "test-env")]
async fn rotate_kek_missing_env_fails() {
    // Ensure deterministic test environment (master KEK present). The test
    // verifies that rotating to a *different* KEK id fails when that specific
    // env isn't set; we still need a base WALLET_ENC_KEY for wallet creation.
    util::set_test_env();
    let (wm, _storage) = setup_manager().await;
    let name = "rot8_missing";
    wm.create_wallet(name, false).await.expect("create wallet");

    // Make sure missing env is not set
    std::env::remove_var("WALLET_ENC_KEY_RED");

    // Attempt rotation should fail due to env not set
    let err = wm.rotate_envelope_kek_for_wallet(name, Some("RED")).await;
    assert!(err.is_err());
}
