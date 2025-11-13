// tests/shamir_tests.rs
//
// Tests for src/crypto/shamir.rs
// - secret splitting and combining
// - different subset reconstruction
// - error handling for insufficient/invalid shares

use defi_hot_wallet::crypto::shamir::{combine_shares, split_secret, ShamirError};
use rand_core::{OsRng, RngCore};

#[test]
fn test_split_and_combine_basic_success() {
    let mut secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    OsRng.fill_bytes(&mut secret_vec);
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    let threshold = 3;
    let shares_count = 5;

    let shares = split_secret(secret, threshold, shares_count).unwrap();
    assert_eq!(shares.len(), shares_count as usize);

    let combination: Vec<(u8, [u8; 32])> =
        shares.iter().take(threshold as usize).copied().collect();
    let recovered_secret = combine_shares(&combination, threshold).unwrap();

    assert_eq!(secret, recovered_secret);
}

#[test]
fn test_split_and_combine_with_different_subset() {
    let mut secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    OsRng.fill_bytes(&mut secret_vec);
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    let threshold = 3;
    let shares_count = 5;

    let shares = split_secret(secret, threshold, shares_count).unwrap();

    let combination = vec![shares[1], shares[3], shares[4]];
    let recovered_secret = combine_shares(&combination, threshold).unwrap();

    assert_eq!(secret, recovered_secret);
}

#[test]
fn test_combine_with_insufficient_shares_produces_error() {
    let mut secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    OsRng.fill_bytes(&mut secret_vec);
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    let threshold = 3;
    let shares_count = 5;

    let shares = split_secret(secret, threshold, shares_count).unwrap();

    let combination: Vec<(u8, [u8; 32])> =
        shares.iter().take((threshold - 1) as usize).copied().collect();
    let result = combine_shares(&combination, threshold);
    assert!(result.is_err());
}

#[test]
fn test_split_with_invalid_parameters() {
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");
    let result = split_secret(secret, 4, 3); // threshold > shares_count -> should error
    assert!(result.is_err());
}

#[test]
fn test_combine_with_no_shares() {
    let parts: Vec<(u8, [u8; 32])> = vec![];
    let result = combine_shares(&parts, 2);
    assert!(result.is_err());
}

#[test]
fn test_split_with_threshold_one() {
    let secret_vec: Vec<u8> = std::iter::repeat_n(1u8, 32).collect();
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");
    let shares = split_secret(secret, 1, 1).unwrap();
    assert_eq!(shares.len(), 1);
    let recovered = combine_shares(&shares, 1).unwrap();
    assert_eq!(recovered, secret);
}

#[test]
fn test_combine_with_duplicate_shares() {
    let secret_vec: Vec<u8> = std::iter::repeat_n(2u8, 32).collect();
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");
    let shares = split_secret(secret, 3, 5).unwrap();
    let combination = vec![shares[0], shares[0], shares[1]];
    let result = combine_shares(&combination, 3);
    assert!(result.is_err());
    if let Err(ShamirError::InvalidParameters(msg)) = result {
        assert!(msg.contains("duplicate share id found"));
    } else {
        panic!("Expected InvalidParameters error for duplicate shares");
    }
}
