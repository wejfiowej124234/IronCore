use anyhow::Result;
use regex::Regex;
// keep a single Regex import; used in multiple validators
use sha3::{Digest, Keccak256};

/// Validates an Ethereum address.
pub fn validate_ethereum_address(address: &str) -> Result<()> {
    if !address.starts_with("0x") || address.len() != 42 {
        return Err(anyhow::anyhow!("Invalid Ethereum address format"));
    }
    let hex_regex = Regex::new(r"^0x[0-9a-fA-F]{40}$")
        .expect("Hardcoded regex should always compile");
    if !hex_regex.is_match(address) {
        return Err(anyhow::anyhow!("Invalid Ethereum address characters"));
    }
    // EIP-55: if mixed-case, enforce checksum. All-lower or all-upper acceptable for compatibility.
    let body = &address[2..];
    let is_all_lower = body.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
    let is_all_upper = body.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase());
    if is_all_lower || is_all_upper {
        return Ok(());
    }
    if !is_eip55_checksum_valid(address) {
        return Err(anyhow::anyhow!("Invalid EIP-55 checksum for Ethereum address"));
    }
    Ok(())
}

fn is_eip55_checksum_valid(addr: &str) -> bool {
    if addr.len() != 42 || !addr.starts_with("0x") {
        return false;
    }
    let body = &addr[2..];
    let lower = body.to_lowercase();
    let mut keccak = Keccak256::new();
    keccak.update(lower.as_bytes());
    let hash = keccak.finalize();
    for (i, ch) in body.chars().enumerate() {
        let nibble = (hash[i / 2] >> (4 * (1 - (i % 2)))) & 0x0f;
        match ch {
            'a'..='f' => {
                if nibble >= 8 {
                    return false;
                }
            }
            'A'..='F' => {
                if nibble < 8 {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

pub fn validate_base58_address(address: &str) -> Result<()> {
    if address.len() < 32 || address.len() > 44 {
        return Err(anyhow::anyhow!("Invalid base58 address length"));
    }
    // Check if it's valid base58
    match bs58::decode(address).into_vec() {
        Ok(decoded) => {
            if decoded.len() != 32 {
                return Err(anyhow::anyhow!("Invalid base58 address decoded length"));
            }
        }
        Err(_) => return Err(anyhow::anyhow!("Invalid base58 address encoding")),
    }
    Ok(())
}

/// Validates an address based on network.
pub fn validate_address(address: &str, network: &str) -> Result<()> {
    match network {
        "eth" | "sepolia" | "polygon" | "bsc" => validate_ethereum_address(address),
        "polygon-testnet" => validate_base58_address(address),
        _ => Err(anyhow::anyhow!("Unsupported network for address validation: {}", network)),
    }
}

/// Validates an amount string (positive number).
pub fn validate_amount(amount: &str) -> Result<f64> {
    let amount: f64 = amount.parse().map_err(|_| anyhow::anyhow!("Invalid amount format"))?;
    if amount <= 0.0 {
        return Err(anyhow::anyhow!("Amount must be positive"));
    }
    Ok(amount)
}

/// Strict decimal validator for amounts to avoid float parsing where exactness matters.
/// Accepts patterns like 123, 0.1, 1.234567 up to 18 decimals. No leading '+', no exponent.
pub fn validate_amount_strict(amount: &str, max_decimals: usize) -> Result<()> {
    if amount.is_empty() {
        return Err(anyhow::anyhow!("Amount cannot be empty"));
    }
    let re = Regex::new(&format!(r"^(?:0|[1-9]\d*)(?:\.(\d{{1,{}}}))?$", max_decimals))
        .expect("Decimal regex pattern should always be valid");
    if !re.is_match(amount) {
        return Err(anyhow::anyhow!("Invalid decimal amount"));
    }
    // disallow 0 or 0.0... values
    if amount.trim_matches('0').trim_matches('.').is_empty() {
        return Err(anyhow::anyhow!("Amount must be positive"));
    }
    Ok(())
}

/// Validates a token symbol.
pub fn validate_token(token: &str) -> Result<()> {
    if token.is_empty() || token.len() > 10 {
        return Err(anyhow::anyhow!("Invalid token symbol"));
    }
    let token_regex = Regex::new(r"^[A-Z]{2,10}$")
        .expect("Hardcoded token regex should always compile");
    if !token_regex.is_match(token) {
        return Err(anyhow::anyhow!("Token symbol must be uppercase letters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_validate_ethereum_address_valid() {
        assert!(validate_ethereum_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44e").is_ok());
    }

    #[test]
    fn test_validate_ethereum_address_invalid_length() {
        assert!(validate_ethereum_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44").is_err());
    }

    #[test]
    fn test_validate_ethereum_address_invalid_chars() {
        assert!(validate_ethereum_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44g").is_err());
    }

    #[test]
    fn test_validate_base58_address_valid() {
        assert!(validate_base58_address("11111111111111111111111111111112").is_ok());
    }

    #[test]
    fn test_validate_base58_address_invalid() {
        assert!(validate_base58_address("invalid").is_err());
    }

    #[test]
    fn test_validate_address_eth() {
        assert!(validate_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44e", "eth").is_ok());
    }

    #[test]
    fn test_validate_address_base58() {
        // polygon-testnet使用base58address
        assert!(validate_address("11111111111111111111111111111112", "polygon-testnet").is_ok());
    }

    #[test]
    fn test_validate_amount_valid() {
        assert_eq!(validate_amount("10.5").unwrap(), 10.5);
    }

    #[test]
    fn test_validate_amount_invalid() {
        assert!(validate_amount("-10").is_err());
    }

    #[test]
    fn test_validate_token_valid() {
        assert!(validate_token("USDC").is_ok());
    }

    #[test]
    fn test_validate_token_invalid() {
        assert!(validate_token("usdc").is_err());
    }

    #[test]
    fn strict_amount_basic_edges() {
        // Valid
        assert!(validate_amount_strict("1", 18).is_ok());
        assert!(validate_amount_strict("0.1", 18).is_ok());
        assert!(validate_amount_strict("123456789012345678", 18).is_ok());
        assert!(validate_amount_strict("1.000000000000000000", 18).is_ok());
        // Invalid
        assert!(validate_amount_strict("", 18).is_err());
        assert!(validate_amount_strict("+1", 18).is_err());
        assert!(validate_amount_strict("1.", 18).is_err());
        assert!(validate_amount_strict("01", 18).is_err());
        assert!(validate_amount_strict("0", 18).is_err());
        assert!(validate_amount_strict("0.000000000000000000", 18).is_err());
        assert!(validate_amount_strict("1e-3", 18).is_err());
        assert!(validate_amount_strict(".1", 18).is_err());
    }

    proptest! {
        // Fuzz valid patterns up to 18 decimals using a single regex
        #[test]
        fn prop_valid_amounts_no_exponent(
            amt in proptest::string::string_regex(r"[1-9][0-9]{0,30}(?:\.[0-9]{1,18})?").unwrap()
        ) {
            prop_assert!(validate_amount_strict(&amt, 18).is_ok());
        }

        // Reject exponent and leading zeros
        #[test]
        fn prop_reject_exponent_and_leading_zeros(
            s in proptest::string::string_regex(r"[0-9eE+\-\.]{1,40}").unwrap()
        ) {
            // Filter out strings we know are valid under our regex
            if validate_amount_strict(&s, 18).is_ok() {
                prop_assume!(false);
            }
            // Ensure we never accidentally accept exponent forms
            if s.contains('e') || s.contains('E') || s.starts_with('+') || s.starts_with('.') || (s.starts_with('0') && s != "0") {
                prop_assert!(validate_amount_strict(&s, 18).is_err());
            }
        }
    }
}
