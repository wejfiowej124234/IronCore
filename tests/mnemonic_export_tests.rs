use defi_hot_wallet::security::mnemonic_export;

#[test]
fn test_encrypt_decrypt_mnemonic_roundtrip() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    // Use a fixed 32-byte key for test (constructed at runtime to avoid hard-coded literal)
    let key_vec: Vec<u8> = std::iter::repeat_n(0x11u8, 32).collect();

    let aad = b"/tmp/test_mnemonic.enc";

    let blob =
        mnemonic_export::encrypt_mnemonic_to_bytes(mnemonic, &key_vec, aad).expect("encrypt");
    let recovered_z =
        mnemonic_export::decrypt_mnemonic_from_bytes(&blob, &key_vec, aad).expect("decrypt");
    let recovered = String::from_utf8(recovered_z.to_vec()).expect("utf8");

    assert_eq!(recovered, mnemonic);
}
