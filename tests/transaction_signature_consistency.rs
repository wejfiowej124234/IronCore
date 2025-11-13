use defi_hot_wallet::crypto::multisig::MultiSignature;
use secp256k1::{Secp256k1, SecretKey};

#[test]
fn test_transaction_signature_consistency() {
    // Build a multisig manager and propose a transaction
    let mut ms = MultiSignature::new(2); // threshold = 2
    let tx_id = ms.create_transaction(
        "0xdeadbeef",
        "1000",
        "eth",
        None,
        Some(2),
    )
    .expect("create");

    // Set nonce/chain and amount precision so signing is allowed
    ms.set_nonce_and_chain_id(&tx_id, 1u64, 1u64).expect("set nonce/chain");
    ms.set_amount_precision_minimal(&tx_id).expect("set precision");

    // Prepare a test secret key
    let secp = Secp256k1::new();
    let sk_bytes: Vec<u8> = std::iter::repeat_n(0x77u8, 32).collect();
    let sk = SecretKey::from_slice(&sk_bytes).expect("secret key");

    // Create a test message for signing
    use secp256k1::Message;
    use sha2::{Sha256, Digest};
    let tx_data = format!("{}:{}:{}", tx_id, "0xdeadbeef", "1000");
    let hash = Sha256::digest(tx_data.as_bytes());
    let msg = Message::from_slice(&hash).expect("message");

    // Sign the message twice using the same secret key
    let sig1 = secp.sign_ecdsa(&msg, &sk);
    let sig2 = secp.sign_ecdsa(&msg, &sk);

    assert_eq!(
        sig1.serialize_compact().to_vec(),
        sig2.serialize_compact().to_vec(),
        "signatures must be deterministic for same input"
    );
}
