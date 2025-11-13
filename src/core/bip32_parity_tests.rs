#[cfg(test)]
mod tests {
    use crate::core::wallet_manager::WalletManager;
    use crate::core::config::WalletConfig;
    use coins_bip32::xkeys::{XPriv, Parent};

    // Helper to create a WalletManager with default config for tests
    async fn make_wm() -> WalletManager {
        let cfg = WalletConfig::default();
        WalletManager::new(&cfg).await.expect("wallet manager init")
    }

    #[tokio::test]
    async fn eth_bip32_parity_zero_seed() {
        let wm = make_wm().await;
        // zero seed 32 bytes
        let seed = vec![0u8; 32];
    let our = wm.test_derive_private_key(&seed, "eth").expect("derive our");
    let our = our.to_vec();

        let mut xprv = XPriv::root_from_seed(&seed, None).expect("root");
        let path = [44 | 0x8000_0000, 60 | 0x8000_0000, 0 | 0x8000_0000, 0, 0];
        for p in path {
            xprv = xprv.derive_child(p).expect("derive child");
        }
        let sk_ref: &k256::ecdsa::SigningKey = xprv.as_ref();
        let lib_bytes = sk_ref.to_bytes();

        assert_eq!(our, lib_bytes.to_vec());
    }

    #[tokio::test]
    async fn eth_bip32_parity_nonzero_seed() {
        let wm = make_wm().await;
        let seed = vec![0x11u8; 32];
    let our = wm.test_derive_private_key(&seed, "eth").expect("derive our");
    let our = our.to_vec();

        let mut xprv = XPriv::root_from_seed(&seed, None).expect("root");
        let path = [44 | 0x8000_0000, 60 | 0x8000_0000, 0 | 0x8000_0000, 0, 0];
        for p in path {
            xprv = xprv.derive_child(p).expect("derive child");
        }
        let sk_ref: &k256::ecdsa::SigningKey = xprv.as_ref();
        let lib_bytes = sk_ref.to_bytes();

        assert_eq!(our, lib_bytes.to_vec());
    }
}
