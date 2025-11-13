use ed25519_dalek::Signer as _;
use ed25519_dalek::SigningKey as Ed25519SigningKey;
use secp256k1::ecdsa::{RecoverableSignature, Signature};
use secp256k1::{Message, Secp256k1, SecretKey};

#[test]
fn test_secp256k1_ecdsa_deterministic() {
    let secp = Secp256k1::new();
    let sk_bytes: Vec<u8> = std::iter::repeat_n(0x11u8, 32).collect();
    let msg_bytes: Vec<u8> = std::iter::repeat_n(0x22u8, 32).collect();
    let sk = SecretKey::from_slice(&sk_bytes).expect("secret key");
    let msg = Message::from_slice(&msg_bytes).expect("msg");

    let sig1: Signature = secp.sign_ecdsa(&msg, &sk);
    let sig2: Signature = secp.sign_ecdsa(&msg, &sk);

    assert_eq!(
        sig1.serialize_compact().to_vec(),
        sig2.serialize_compact().to_vec(),
        "secp256k1 ECDSA signatures must be deterministic for same key+msg"
    );
}

#[test]
fn test_secp256k1_recoverable_deterministic() {
    let secp = Secp256k1::new();
    let sk_bytes2: Vec<u8> = std::iter::repeat_n(0x33u8, 32).collect();
    let msg_bytes2: Vec<u8> = std::iter::repeat_n(0x44u8, 32).collect();
    let sk = SecretKey::from_slice(&sk_bytes2).expect("secret key");
    let msg = Message::from_slice(&msg_bytes2).expect("msg");

    let r1: RecoverableSignature = secp.sign_ecdsa_recoverable(&msg, &sk);
    let r2: RecoverableSignature = secp.sign_ecdsa_recoverable(&msg, &sk);

    let (rec1, comp1) = r1.serialize_compact();
    let (rec2, comp2) = r2.serialize_compact();

    assert_eq!(comp1.to_vec(), comp2.to_vec(), "recoverable signature compact must match");
    assert_eq!(rec1.to_i32(), rec2.to_i32(), "recoverable signature recovery id must match");
}

#[test]
fn test_ed25519_deterministic() {
    // ed25519 is deterministic by design
    // ed25519-dalek v2 SigningKey::from_bytes accepts a 32-byte array directly
    let seed_vec: Vec<u8> = std::iter::repeat_n(0x55u8, 32).collect();
    let seed: [u8; 32] = seed_vec.clone().try_into().expect("32 bytes");
    let sk = Ed25519SigningKey::from_bytes(&seed);
    let msg = b"fixed message for ed25519";

    let sig1 = sk.sign(msg);
    let sig2 = sk.sign(msg);

    assert_eq!(
        sig1.to_bytes().to_vec(),
        sig2.to_bytes().to_vec(),
        "ed25519 signatures must be deterministic"
    );
}
