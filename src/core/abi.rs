use anyhow::Result;
use sha3::{Digest, Keccak256};

/// Compute the first 4 bytes (function selector) from a signature string, e.g. "transfer(address,uint256)".
pub fn selector_from_signature(signature: &str) -> [u8; 4] {
    let mut keccak = Keccak256::new();
    keccak.update(signature.as_bytes());
    let out = keccak.finalize();
    [out[0], out[1], out[2], out[3]]
}

/// Encode an Ethereum address (20-byte hex, with or without 0x) into a 32-byte ABI word (left-padded).
pub fn abi_word_address(addr_hex: &str) -> Result<[u8; 32]> {
    let addr = addr_hex.strip_prefix("0x").unwrap_or(addr_hex);
    if addr.len() != 40 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!("Invalid Ethereum address hex"));
    }
    let mut out = [0u8; 32];
    for i in 0..20 {
        out[12 + i] = u8::from_str_radix(&addr[2 * i..2 * i + 2], 16)
            .map_err(|_| anyhow::anyhow!("Invalid hex in address"))?;
    }
    Ok(out)
}

/// Encode a decimal string into a 32-byte big-endian unsigned ABI word.
pub fn abi_word_uint256_from_str(value: &str) -> Result<[u8; 32]> {
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow::anyhow!("Uint256 must be a non-empty integer string"));
    }
    let v = value
        .parse::<u128>()
        .map_err(|_| anyhow::anyhow!("Uint256 value out of supported range (<= u128 for now)"))?;
    let mut out = [0u8; 32];
    out[16..].copy_from_slice(&v.to_be_bytes());
    Ok(out)
}

/// Pack a selector and ABI words contiguously into calldata.
pub fn abi_pack(selector: [u8; 4], words: &[[u8; 32]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 32 * words.len());
    out.extend_from_slice(&selector);
    for w in words {
        out.extend_from_slice(w);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_from_signature() {
        // transfer(address,uint256) -> a9059cbb
        let sel = selector_from_signature("transfer(address,uint256)");
        assert_eq!(sel, [0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_abi_word_address_padding() {
        let word = abi_word_address("0x1111111111111111111111111111111111111111").unwrap();
        assert!(word[..12].iter().all(|&b| b == 0));
        assert!(word[12..].iter().all(|&b| b == 0x11));
        // without 0x
        let word2 = abi_word_address("1111111111111111111111111111111111111111").unwrap();
        assert_eq!(word, word2);
    }

    #[test]
    fn test_abi_word_uint256_from_str() {
        let word = abi_word_uint256_from_str("42").unwrap();
        assert!(word[..31].iter().all(|&b| b == 0));
        assert_eq!(word[31], 42);
        let big = abi_word_uint256_from_str("340282366920938463463374607431768211455"); // u128::MAX
        assert!(big.is_ok());
        let err = abi_word_uint256_from_str("").unwrap_err();
        assert!(err.to_string().contains("non-empty"));
        let err = abi_word_uint256_from_str("1.0").unwrap_err();
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn test_abi_pack() {
        let selector = selector_from_signature("approve(address,uint256)");
        let addr = abi_word_address("0x2222222222222222222222222222222222222222").unwrap();
        let amt = abi_word_uint256_from_str("1000").unwrap();
        let data = abi_pack(selector, &[addr, amt]);
        assert_eq!(data.len(), 4 + 64);
        assert_eq!(&data[0..4], &selector);
        assert_eq!(&data[4..36], &addr);
        assert_eq!(&data[36..68], &amt);
    }
}
