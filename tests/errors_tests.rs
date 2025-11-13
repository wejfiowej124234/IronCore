// ...existing code...
use defi_hot_wallet::core::errors::WalletError;
use serde_json::Value;

#[test]
fn all_variants_display_and_conversions() {
    let cases = vec![
        (WalletError::ConfigError("cfg".into()), "Configuration error: cfg"),
        (WalletError::StorageError("db".into()), "Storage error: db"),
        (WalletError::BlockchainError("bc".into()), "Blockchain error: bc"),
        (WalletError::CryptoError("c".into()), "Crypto error: c"),
        (WalletError::BridgeError("b".into()), "Bridge error: b"),
        (WalletError::ValidationError("v".into()), "Validation error: v"),
        (WalletError::NetworkError("n".into()), "Network error: n"),
        (WalletError::MnemonicError("m".into()), "Mnemonic error: m"),
        (WalletError::KeyDerivationError("k".into()), "Key derivation error: k"),
        (WalletError::AddressError("a".into()), "Address error: a"),
        (WalletError::SerializationError("s".into()), "Serialization error: s"),
        (WalletError::Other("o".into()), "Error: o"),
    ];
    for (err, expect) in cases {
        assert_eq!(format!("{}", err), expect);
    }

    // From<std::io::Error>
    let io_err = std::io::Error::other("io fail");
    let w: WalletError = io_err.into();
    match w {
        WalletError::StorageError(msg) => assert!(msg.contains("io fail")),
        _ => panic!("expected StorageError"),
    }

    // From<serde_json::Error>
    let sj = serde_json::from_str::<Value>("not json").unwrap_err();
    let w2: WalletError = sj.into();
    match w2 {
        WalletError::ValidationError(msg) => assert!(!msg.is_empty()),
        _ => panic!("expected ValidationError"),
    }

    // From<anyhow::Error>
    let a = anyhow::anyhow!("anyhow-msg");
    let w3: WalletError = a.into();
    match w3 {
        WalletError::Other(msg) => assert!(msg.contains("anyhow-msg")),
        _ => panic!("expected Other"),
    }
}
