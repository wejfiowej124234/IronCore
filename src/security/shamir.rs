use rand::Rng;
use thiserror::Error;

/// Shamir secret sharing related error types for the security layer.
#[derive(Debug, Error)]
pub enum ShamirError {
    #[error("invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("split failed: {0}")]
    SplitFailed(String),

    #[error("combine failed: {0}")]
    CombineFailed(String),
}

// GF(2^8) constants and arithmetic (AES polynomial 0x11b -> 0x1b variant)
const POLY: u8 = 0x1b;

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut r: u8 = 0;
    while b != 0 {
        if (b & 1) != 0 {
            r ^= a;
        }
        let hi = (a & 0x80) != 0;
        a <<= 1;
        if hi {
            a ^= POLY;
        }
        b >>= 1;
    }
    r
}

fn gf_pow(mut a: u8, mut e: u8) -> u8 {
    if e == 0 {
        return 1;
    }
    let mut r = 1u8;
    while e != 0 {
        if (e & 1) != 0 {
            r = gf_mul(r, a);
        }
        a = gf_mul(a, a);
        e >>= 1;
    }
    r
}

fn gf_inv(a: u8) -> u8 {
    if a == 0 {
        panic!("gf_inv(0)");
    }
    gf_pow(a, 0xfe)
}

fn eval_poly_at(coeffs: &[u8], x: u8) -> u8 {
    let mut result = 0u8;
    let mut xp = 1u8;
    for &c in coeffs.iter() {
        result ^= gf_mul(c, xp);
        xp = gf_mul(xp, x);
    }
    result
}

/// Split a 32-byte secret into `total_shares` byte-array shares with threshold `threshold`.
pub fn split_secret<S: AsRef<[u8]>>(
    secret: S,
    threshold: u8,
    total_shares: u8,
) -> Result<Vec<(u8, [u8; 32])>, ShamirError> {
    use std::num::NonZeroU8;
    let s = secret.as_ref();
    let k = NonZeroU8::new(threshold)
        .ok_or_else(|| ShamirError::InvalidParameters("Threshold cannot be zero".to_string()))?;
    let n = NonZeroU8::new(total_shares)
        .ok_or_else(|| ShamirError::InvalidParameters("Total shares cannot be zero".to_string()))?;
    if k > n {
        return Err(ShamirError::InvalidParameters(
            "Threshold cannot be greater than total shares".to_string(),
        ));
    }
    if s.len() != 32 {
        return Err(ShamirError::InvalidParameters("Secret must be exactly 32 bytes".to_string()));
    }

    // build per-byte polynomials
    let mut rng = rand::rngs::OsRng;
    let mut coeffs_per_byte: Vec<Vec<u8>> = Vec::with_capacity(32);
    for &b in s.iter().take(32) {
        let mut coeffs = vec![0u8; threshold as usize];
        coeffs[0] = b;
        for coeff in coeffs.iter_mut().skip(1) {
            *coeff = rng.gen();
        }
        coeffs_per_byte.push(coeffs);
    }

    let mut shares: Vec<(u8, [u8; 32])> = Vec::with_capacity(total_shares as usize);
    for id in 1..=total_shares {
        let x = id;
        let mut payload = [0u8; 32];
        for byte_idx in 0..32 {
            payload[byte_idx] = eval_poly_at(&coeffs_per_byte[byte_idx], x);
        }
        shares.push((id, payload));
    }
    Ok(shares)
}

/// Combine shares (id, [u8;32]) using Lagrange interpolation in GF(2^8).
pub fn combine_shares(shares: &[(u8, [u8; 32])], threshold: u8) -> Result<[u8; 32], ShamirError> {
    if shares.is_empty() {
        return Err(ShamirError::InvalidParameters("shares must not be empty".to_string()));
    }
    if threshold == 0 {
        return Err(ShamirError::InvalidParameters("threshold cannot be zero".to_string()));
    }
    if shares.len() < threshold as usize {
        return Err(ShamirError::CombineFailed("Insufficient shares for recovery".to_string()));
    }

    // validate ids unique and non-zero
    let mut ids = std::collections::HashSet::new();
    for (id, _payload) in shares.iter() {
        if *id == 0 {
            return Err(ShamirError::InvalidParameters("share id cannot be zero".to_string()));
        }
        if !ids.insert(*id) {
            return Err(ShamirError::InvalidParameters(format!(
                "duplicate share id found: {}",
                id
            )));
        }
    }

    // Lagrange interpolation per-byte
    let k = threshold as usize;
    let xs: Vec<u8> = shares.iter().map(|(id, _)| *id).collect();
    let mut secret = [0u8; 32];

    for (byte_idx, secret_byte) in secret.iter_mut().enumerate().take(32) {
        let mut acc = 0u8;
        for j in 0..k {
            let xj = xs[j];
            let yj = shares[j].1[byte_idx];

            let mut num = 1u8;
            let mut den = 1u8;
            for (m, &xm) in xs.iter().enumerate().take(k) {
                if m == j {
                    continue;
                }
                num = gf_mul(num, xm);
                let diff = xm ^ xj;
                if diff == 0 {
                    return Err(ShamirError::CombineFailed(
                        "Invalid share x difference zero".to_string(),
                    ));
                }
                den = gf_mul(den, diff);
            }
            let inv_den = gf_inv(den);
            let lj = gf_mul(num, inv_den);
            let term = gf_mul(yj, lj);
            acc ^= term;
        }
        *secret_byte = acc;
    }

    Ok(secret)
}

/// Compatibility alias
pub fn combine_secret(shares: &[(u8, [u8; 32])], threshold: u8) -> Result<[u8; 32], ShamirError> {
    combine_shares(shares, threshold)
}
