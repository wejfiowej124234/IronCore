//! tests/blockchain_ethereum_address_tests.rs

use ethers::types::Address;
use std::str::FromStr;

/// Normalize input and validate Ethereum address.
/// - Accepts inputs with or without "0x"/"0X" prefix.
/// - Normalizes prefix to lowercase "0x" before parsing so addresses like "0X..." are accepted.
fn validate_address(s: &str) -> bool {
    // Strip optional 0x/0X prefix, then re-add lowercase "0x" to normalize.
    let rest =
        if s.len() >= 2 && (s.starts_with("0x") || s.starts_with("0X")) { &s[2..] } else { s };

    // Quick length check: Ethereum address hex (without prefix) must be 40 chars.
    if rest.len() != 40 {
        return false;
    }

    let normalized = format!("0x{}", rest);
    Address::from_str(&normalized).is_ok()
}

#[test]
fn test_validate_address_valid() {
    let valid_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert!(validate_address(valid_address));
}

#[test]
fn test_validate_address_invalid_short() {
    assert!(!validate_address("0x12345"));
}

#[test]
fn test_validate_address_valid_no_prefix() {
    assert!(validate_address("742d35Cc6634C0532925a3b844Bc454e4438f44e"));
}

#[test]
fn test_validate_address_invalid_special_chars() {
    assert!(!validate_address("0x742d35Cc6634C0532925a3b844Bc454e4438f44e!"));
}

#[test]
fn test_validate_address_empty() {
    assert!(!validate_address(""));
}

#[test]
fn test_validate_address_all_zeros() {
    let zero_address = "0x0000000000000000000000000000000000000000";
    assert!(validate_address(zero_address));
}

#[test]
fn test_validate_address_case_insensitive() {
    let lower = "0x742d35cc6634c0532925a3b844bc454e4438f44e";
    let upper = "0x742D35CC6634C0532925A3B844BC454E4438F44E";
    assert!(validate_address(lower));
    assert!(validate_address(upper));
}

#[test]
fn test_validate_address_too_long() {
    let long_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e1234";
    assert!(!validate_address(long_address));
}

#[test]
fn test_validate_address_too_short() {
    let short_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44";
    assert!(!validate_address(short_address));
}

#[test]
fn test_validate_address_with_checksum() {
    let checksum_address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert!(validate_address(checksum_address));
}

#[test]
fn test_validate_address_mixed_case_valid() {
    let mixed_case = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    assert!(validate_address(mixed_case));
}

#[test]
fn test_validate_address_uppercase_valid() {
    let uppercase = "0X742D35CC6634C0532925A3B844BC454E4438F44E";
    // Normalize "0X" to "0x" and accept uppercase hex digits.
    assert!(validate_address(uppercase));
}

#[test]
fn test_validate_address_with_numbers_only() {
    let num_address = "0x1234567890123456789012345678901234567890";
    assert!(validate_address(num_address));
}

#[test]
fn test_validate_address_with_leading_zeros() {
    let leading_zero = "0x0000000000000000000000000000000000000000";
    assert!(validate_address(leading_zero));
}
