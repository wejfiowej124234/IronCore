use defi_hot_wallet::core::wallet::create;
use defi_hot_wallet::crypto::kdf::KeyDerivation;
use defi_hot_wallet::security::SecretVec;

#[tokio::test]
async fn test_derive_master_key_returns_secretvec() {
    // Known mnemonic (BIP39 test vector)
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let key: SecretVec =
        create::derive_master_key(mnemonic).await.expect("derive_master_key should succeed");

    // SecretVec is a Zeroizing<Vec<u8>> alias; ensure length is 32
    assert_eq!(key.len(), 32);
}

#[test]
fn test_hkdf_returns_zeroizing_vec() {
    let kdf = KeyDerivation::hkdf();
    let ikm = b"input_key_material";
    let salt = b"some_salt";

    let key = kdf.derive_key(ikm, salt, 32).expect("HKDF derive should succeed");

    // key is Zeroizing<Vec<u8>>; check length and deterministic behavior
    assert_eq!(key.len(), 32);
    // Re-run to confirm deterministic output for same inputs
    let key2 = kdf.derive_key(ikm, salt, 32).expect("HKDF derive should succeed");
    assert_eq!(key.as_slice(), key2.as_slice());
}
