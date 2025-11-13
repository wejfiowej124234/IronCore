//! # Shamir 秘密分享 (Shamir's Secret Sharing)
//!
//! ## 概要
//!
//! 实现一个基于 GF(2^8) 的 Shamir 秘密分享层，用于把敏感数据分割成多份份额并按阈值恢复。
//! 对阈值 ≥ 1 直接对原始 secret 的每个字节进行多项式分割/插值（可变长度，最大 255 字节）；
//! 阈值 == 1 时直接复用原始 secret（不做多项式扩散）。
//!
//! ## 提供的主要函数 (API)
//!
//! - `split_secret(secret: &[u8], threshold: u8, total_shares: u8) -> Result<Vec<Vec<u8>>, ShamirError>`
//! - `combine_shares(shares: &[Vec<u8>]) -> Result<Vec<u8>, ShamirError>`
//! - `combine_secret(...)` 别名。
//!
//! ## Share 二进制格式 (必须严格遵守)
//!
//! - byte 0: `threshold`
//! - byte 1: `share id` (1..=total_shares)
//! - byte 2..: `payload`
//!   - 若 `threshold == 1`：payload = 原始 secret（长度可变）
//!   - 若 `threshold > 1`：payload = 与原始 secret 等长的 y 值序列（对每个字节独立构造多项式并在 x=id 处求值）
//!
//! ## 运作原理 (要点)
//!
//! - 对 `threshold > 1`：对原始 secret 的每个字节分别构造 `degree = threshold-1` 的随机多项式，计算 `y = P_i(x)`（x = share id），每个 share 放入各字节 y 值。
//! - 恢复时对每个字节在 x=0 处做 Lagrange 插值（在 GF(2^8) 上，域运算由 `gf_mul`/`gf_inv`/`gf_pow` 实现）；插值结果就是原始 secret（可变长度）。
//! - 为避免实现不一致，share 格式与 id 唯一性、长度校验严格检查。
//!
//! ## 设计决策与注意事项
//!
//! - 为确保与调用方直觉一致，本实现 `combine` 总是恢复原始 secret（而不是哈希材料）。
//! - `share id` 从 1 开始且不能重复；差值为 0 会导致插值失败（检测并报错）。
//! - 对参数做严格校验（`threshold`/`total_shares` 不能为 0，且 `threshold <= total_shares`；合并时要提供 `≥ threshold` 份额）。
//! - `gf` 运算基于 AES 多项式 (0x11b) 的移位/异或实现（常见 GF(2^8) 实现）。
//! - 若其它模块或测试期望不同的 share 编码，请统一约定或在外层做兼容层。
//!
//! ## 如何调用 (示例)
//!
//! ```rust
//! # use defi_hot_wallet::shamir::{split_secret, combine_shares};
//! let secret = b"hello";
//! // 将秘密 "hello" 拆分为 5 份，需要 3 份才能恢复
//! let shares = split_secret(secret, 3, 5).unwrap(); // Vec<Vec<u8>>
//!
//! // 从 5 份中任取 3 份
//! let subset = vec![shares[0].clone(), shares[2].clone(), shares[4].clone()];
//!
//! // 恢复原始秘密
//! let recovered = combine_shares(&subset).unwrap(); // Zeroizing<Vec<u8>> (zeroized on drop)
//! assert_eq!(recovered.as_slice(), secret.as_slice());
//! ```
use crate::security::SecretVec;
use rand::Rng;
use std::num::NonZeroU8;

/// Shamir 错误类型
#[derive(Debug, thiserror::Error)]
pub enum ShamirError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Failed to split secret: {0}")]
    SplitFailed(String),
    #[error("Failed to combine shares: {0}")]
    CombineFailed(String),
}

/// GF(2^8) 常量（AES 多项式 0x11b -> 0x1b 用于移位异或实现）
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
    // a^(254) in GF(2^8)
    gf_pow(a, 0xfe)
}

/// Evaluate polynomial with little-endian coeffs at x (coeffs[0] + coeffs[1]*x + ...)
fn eval_poly_at(coeffs: &[u8], x: u8) -> u8 {
    let mut result = 0u8;
    let mut xp = 1u8;
    for &c in coeffs.iter() {
        result ^= gf_mul(c, xp);
        xp = gf_mul(xp, x);
    }
    result
}

/// Split secret into `total_shares` shares with threshold `threshold`.
/// Share format:
/// - threshold == 1: each share = [threshold, id, payload... (original secret bytes)]
/// - threshold > 1 : each share = [threshold, id, 32-byte payload (sha256(secret) split)]
pub fn split_secret(
    secret: &[u8],
    threshold: u8,
    total_shares: u8,
) -> Result<Vec<SecretVec>, ShamirError> {
    let k = NonZeroU8::new(threshold)
        .ok_or_else(|| ShamirError::InvalidParameters("Threshold cannot be zero".to_string()))?;
    let n = NonZeroU8::new(total_shares)
        .ok_or_else(|| ShamirError::InvalidParameters("Total shares cannot be zero".to_string()))?;

    if k > n {
        return Err(ShamirError::InvalidParameters(
            "Threshold cannot be greater than total shares".to_string(),
        ));
    }

    // Special-case: threshold == 1 -> replicate raw secret
    if threshold == 1 {
        let mut result = Vec::with_capacity(total_shares as usize);
        for i in 1..=total_shares {
            let mut share = Vec::with_capacity(2 + secret.len());
            share.push(threshold);
            share.push(i);
            share.extend_from_slice(secret);
            result.push(SecretVec::new(share));
        }
        return Ok(result);
    }

    // General case: operate on raw secret bytes (no SHA-256). This makes payload length == secret.len()
    if secret.is_empty() {
        return Err(ShamirError::InvalidParameters("Secret cannot be empty".to_string()));
    }
    let secret_len = secret.len();
    if secret_len > 255 {
        return Err(ShamirError::InvalidParameters("Secret too long; max 255 bytes".to_string()));
    }

    // For each byte of the secret, build a random polynomial of degree threshold-1
    let mut rng = rand::rngs::OsRng;
    let mut coeffs_per_byte: Vec<Vec<u8>> = Vec::with_capacity(secret_len);
    for &b in secret.iter().take(secret_len) {
        let mut coeffs = vec![0u8; threshold as usize];
        coeffs[0] = b;
        for coeff in coeffs.iter_mut().skip(1) {
            *coeff = rng.gen();
        }
        coeffs_per_byte.push(coeffs);
    }

    // Build shares: for id = 1..=total_shares compute payload bytes
    let mut shares: Vec<SecretVec> = Vec::with_capacity(total_shares as usize);
    for id in 1..=total_shares {
        let x = id;
        let mut share = Vec::with_capacity(2 + secret_len);
        share.push(threshold);
        share.push(id);
        // The length of the payload is now secret_len, not a fixed 32 bytes.
        // The payload itself contains the y-values for each byte of the original secret.
        for coeffs in coeffs_per_byte.iter().take(secret_len) {
            let y = eval_poly_at(coeffs, x);
            share.push(y);
        }
        shares.push(SecretVec::new(share));
    }

    Ok(shares)
}

/// Combine shares and recover secret material.
/// - If threshold == 1 returns raw payload bytes
/// - If threshold > 1 expects shares to be [threshold, id, 32-byte payload] and returns 32-byte secret (sha256(secret))
pub fn combine_shares(shares: &[SecretVec]) -> Result<SecretVec, ShamirError> {
    if shares.is_empty() {
        return Err(ShamirError::InvalidParameters("Shares cannot be empty".to_string()));
    }

    // validate threshold consistency and unique/non-zero ids
    let mut ids = std::collections::HashSet::new();
    let mut threshold: u8 = 0;
    for s in shares {
        let s_slice = s.as_slice();
        if s_slice.len() < 3 {
            // must have at least threshold, id, and one payload byte
            return Err(ShamirError::InvalidParameters(
                "Invalid share format: too short".to_string(),
            ));
        }
        let th = s_slice[0];
        let id = s_slice[1];
        if th == 0 {
            return Err(ShamirError::InvalidParameters(
                "Invalid threshold in share: zero".to_string(),
            ));
        }
        if id == 0 {
            return Err(ShamirError::InvalidParameters("Invalid share id: zero".to_string()));
        }
        if threshold == 0 {
            threshold = th;
        } else if threshold != th {
            return Err(ShamirError::InvalidParameters(
                "Inconsistent threshold in shares".to_string(),
            ));
        }
        if !ids.insert(id) {
            return Err(ShamirError::InvalidParameters(format!("Duplicate share ID: {}", id)));
        }
    }

    // threshold == 1: return payload of first share
    if threshold == 1 {
        let payload = shares[0].as_slice()[2..].to_vec();
        return Ok(SecretVec::new(payload));
    }

    // determine payload length from first share and validate all shares have the same payload length
    let payload_len = shares[0].as_slice().len() - 2; // safe: earlier check ensures len>=3
    if payload_len == 0 {
        return Err(ShamirError::InvalidParameters(
            "Invalid share payload length: zero".to_string(),
        ));
    }
    for s in shares {
        if s.as_slice().len() - 2 != payload_len {
            return Err(ShamirError::InvalidParameters(
                "Inconsistent share payload length".to_string(),
            ));
        }
    }

    let t = shares.len(); // number of shares provided, must be >= threshold
    if (t as u8) < threshold {
        return Err(ShamirError::CombineFailed("Insufficient shares for threshold".to_string()));
    }
    let xs: Vec<u8> = shares.iter().map(|s| s.as_slice()[1]).collect();
    let mut secret = vec![0u8; payload_len];

    for (byte_idx, secret_byte) in secret.iter_mut().enumerate().take(payload_len) {
        let mut acc = 0u8;
        for j in 0..t {
            let xj = xs[j];
            let yj = shares[j].as_slice()[2 + byte_idx];

            // compute numerator = prod_{m!=j} xm
            // compute denom = prod_{m!=j} (xm ^ xj)
            let mut num = 1u8;
            let mut den = 1u8;
            for (m, &xm) in xs.iter().enumerate().take(t) {
                if m == j {
                    continue;
                }
                num = gf_mul(num, xm);
                let diff = xm ^ xj; // subtraction in GF(2^8) is xor
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

    Ok(SecretVec::new(secret))
}

// API alias
/// Alias for `combine_shares` to maintain API compatibility.
pub fn combine_secret(shares: &[SecretVec]) -> Result<SecretVec, ShamirError> {
    combine_shares(shares)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_and_combine() {
        let secret = b"test secret data";
        let shares = split_secret(secret, 3, 5).unwrap();
        assert_eq!(shares.len(), 5);
        let recovered =
            combine_shares(&[shares[0].clone(), shares[2].clone(), shares[4].clone()]).unwrap();
        // With the fix, we recover the original secret directly
        assert_eq!(&*recovered, secret);
    }

    #[test]
    fn test_insufficient_shares() {
        let secret = b"test";
        let shares = split_secret(secret, 3, 5).unwrap();
        assert!(combine_shares(&shares[..2]).is_err());
    }

    #[test]
    fn test_invalid_shares() {
        assert!(combine_shares(&[]).is_err());
    }

    #[test]
    fn test_min_threshold() {
        let secret = b"min";
        let shares = split_secret(secret, 1, 1).unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].as_slice().len(), 2 + secret.len());
        let recovered = combine_shares(&shares).unwrap();
        assert_eq!(&*recovered, secret);
    }

    // Additional tests

    #[test]
    fn test_combine_with_more_than_threshold() {
        let secret = b"more shares test";
        let shares = split_secret(secret, 3, 5).unwrap();
        let subset = vec![shares[0].clone(), shares[1].clone(), shares[3].clone()];
        let recovered = combine_shares(&subset).unwrap();
        assert_eq!(&*recovered, secret);
    }

    #[test]
    fn test_all_combinations_threshold_3_of_5() {
        // Exhaustively test all 3-of-5 combinations for a deterministic secret
        let secret = b"deterministic secret for combos";
        let shares = split_secret(secret, 3, 5).unwrap();

        for i in 0..5 {
            for j in (i + 1)..5 {
                for k in (j + 1)..5 {
                    let subset = vec![shares[i].clone(), shares[j].clone(), shares[k].clone()];
                    let recovered = combine_shares(&subset).unwrap();
                    assert_eq!(&*recovered, secret, "failed for combo {},{},{}", i, j, k);
                }
            }
        }
    }

    #[test]
    fn test_duplicate_share_id_error() {
        let secret = b"dup id test";
        let mut shares = split_secret(secret, 3, 5).unwrap();
        // duplicate the first share id into second share to trigger duplicate id error
        // convert to mutable Vec<u8> to modify
        let s0 = shares[0].as_slice().to_vec();
        let mut s1 = shares[1].as_slice().to_vec();
        s1[1] = s0[1];
        shares[1] = SecretVec::new(s1);
        shares[0] = SecretVec::new(s0);
        let res = combine_shares(&shares[..3]);
        assert!(matches!(res, Err(ShamirError::InvalidParameters(_))));
    }

    #[test]
    fn test_invalid_threshold_zero_split() {
        let secret = b"zero";
        let res = split_secret(secret, 0, 5);
        assert!(matches!(res, Err(ShamirError::InvalidParameters(_))));
    }

    #[test]
    fn test_invalid_total_zero_split() {
        let secret = b"zero";
        let res = split_secret(secret, 2, 0);
        assert!(matches!(res, Err(ShamirError::InvalidParameters(_))));
    }

    #[test]
    fn test_malformed_share_length_on_combine() {
        // Create a malformed share for threshold > 1 (wrong payload length)
        let mut shares = split_secret(b"some secret", 2, 3).unwrap();
        // Ensure we actually shorten the first share so it's malformed regardless of secret length
        // Choose a small payload length (e.g. 5) to guarantee truncation for typical test secrets
        let mut s0 = shares[0].as_slice().to_vec();
        s0.truncate(2 + 5); // wrong length
        shares[0] = SecretVec::new(s0);
        let res = combine_shares(&shares);
        assert!(matches!(res, Err(ShamirError::InvalidParameters(_))));
    }
}
