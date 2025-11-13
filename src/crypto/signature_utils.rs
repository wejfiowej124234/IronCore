use secp256k1::{self};

/// Ensure ECDSA signature uses low-S value (s <= n/2) to avoid malleability.
pub fn ensure_low_s(compact_sig: &[u8; 64]) -> [u8; 64] {
    // Use secp256k1 to parse and normalize
    if let Ok(mut sig) = secp256k1::ecdsa::Signature::from_compact(compact_sig) {
        // normalize_s mutates signature in-place
        sig.normalize_s();
        // Obtain compact 64-byte representation
        let compact = sig.serialize_compact();
        let mut out = [0u8; 64];
        out.copy_from_slice(&compact);
        out
    } else {
        // Fallback: return as-is
        *compact_sig
    }
}

/// For Ethers style signature (r,s,v) where v may be 27/28 or eip155, return normalized v (27/28)
pub fn normalize_v(v: u64) -> u8 {
    // If v is already 27 or 28, keep. If greater (EIP-155), reduce.
    if v == 27 || v == 28 {
        return v as u8;
    }
    // EIP-155: v = chain_id * 2 + 35 or +36
    // Normalize: v_mod = v % 2 -> 27 + (v % 2)
    let v_mod = (v & 1) as u8;
    27u8 + v_mod
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{Secp256k1, SecretKey};

    #[test]
    fn test_ensure_low_s() {
        // Create a random key and sign a fixed message to get a signature
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[1u8; 32]).expect("secret key");
        let message = secp256k1::Message::from_slice(&[2u8; 32]).expect("msg");
        let sig = secp.sign_ecdsa_recoverable(&message, &sk);
        let (_recid, compact) = sig.serialize_compact();
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&compact);
        let normalized = ensure_low_s(&arr);

        // Parse normalized signature and check s is low (normalize_s makes it so)
        let std_sig = secp256k1::ecdsa::Signature::from_compact(&normalized).expect("sig");
        // There's no direct is_low_s API; rely on normalize_s being idempotent
        let mut clone = std_sig;
        clone.normalize_s();
        assert_eq!(clone.serialize_compact().to_vec(), std_sig.serialize_compact().to_vec());
    }
}
