use defi_hot_wallet::mvp::{
    create_transaction, derive_public_key_from_bytes, is_signature_valid, sign_transaction,
    verify_signature,
};

#[test]
fn tx_signing_roundtrip() {
    let tx = create_transaction();
    // Use a proper 32-byte private key for testing
    let private_key_bytes = [
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
        0xde, 0xf0,
    ];
    let public_key = derive_public_key_from_bytes(&private_key_bytes);
    let signature = sign_transaction(&tx, &private_key_bytes).expect("Failed to sign transaction");

    // signature is a SecretVec (zeroizing Vec<u8>); use slice when checking
    assert!(verify_signature(&tx, signature.as_ref(), &public_key));
    assert!(is_signature_valid(signature.as_ref(), &public_key));
}
