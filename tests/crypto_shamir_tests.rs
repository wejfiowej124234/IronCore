use defi_hot_wallet::crypto::shamir::{combine_secret, combine_shares, split_secret};
use itertools::Itertools;

#[test]
fn test_shamir_secret_sharing_basic() {
    let threshold = 3;
    let shares_count = 5;
    // create a simple [u8; 32] secret
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let mut secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");
    secret[0] = 42;
    secret[1] = 101;
    secret[2] = 53;

    let shares = split_secret(secret, threshold, shares_count).unwrap();
    assert_eq!(shares.len(), shares_count as usize);

    let recovered = combine_secret(&shares[0..threshold as usize], threshold).unwrap();
    assert_eq!(recovered, secret);
}

#[test]
fn test_shamir_insufficient_shares() {
    let threshold = 3;
    let shares_count = 5;
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let mut secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");
    secret[0] = 42;

    let shares = split_secret(secret, threshold, shares_count).unwrap();

    let result = combine_secret(&shares[0..(threshold as usize - 1)], threshold);
    assert!(result.is_err());
}

#[test]
fn test_shamir_invalid_threshold() {
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    // threshold > shares_count should be an error
    let result = split_secret(secret, 5, 3);
    assert!(result.is_err());
}

#[test]
fn test_shamir_zero_threshold() {
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    // zero threshold should be an error
    let result = split_secret(secret, 0, 5);
    assert!(result.is_err());
}

#[test]
fn test_shamir_equal_threshold_and_shares() {
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    // threshold == shares_count should succeed
    let result = split_secret(secret, 3, 3);
    assert!(result.is_ok());
}

#[test]
fn test_shamir_reconstruct_exact() {
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let mut secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");
    secret.iter_mut().enumerate().for_each(|(i, v)| *v = (i * 7) as u8);
    let result = split_secret(secret, 2, 3);

    let shares = result.unwrap();
    let recovered = combine_shares(&shares[0..2], 2).unwrap();
    assert_eq!(recovered, secret);
}

#[test]
fn test_shamir_different_share_subsets() {
    let threshold = 3;
    let shares_count = 5;
    let secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    let mut secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");
    secret.iter_mut().enumerate().take(21).for_each(|(i, v)| *v = (i * 13 + 7) as u8);

    let shares = split_secret(secret, threshold, shares_count).unwrap();

    let recovered = combine_shares(&shares[0..threshold as usize], threshold).unwrap();
    assert_eq!(recovered, secret);

    // test a different subset of shares
    let combination: Vec<(u8, [u8; 32])> = vec![shares[0], shares[2], shares[4]];

    let recovered2 = combine_shares(&combination, threshold).unwrap();
    assert_eq!(recovered2, secret);
}

#[test]
fn test_shamir_all_possible_combinations() {
    let threshold = 3;
    let shares_count = 5;
    let mut secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    secret_vec.iter_mut().enumerate().take(18).for_each(|(i, v)| *v = (i * 11) as u8);
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    let shares = split_secret(secret, threshold, shares_count).unwrap();

    // test all combinations of `threshold` shares
    for combo in shares.iter().combinations(threshold as usize) {
        let selected_shares: Vec<(u8, [u8; 32])> = combo.into_iter().copied().collect();
        let recovered = combine_shares(&selected_shares, threshold).unwrap();
        assert_eq!(recovered, secret);
    }
}

#[test]
fn test_shamir_tampered_share() {
    let threshold = 3;
    let shares_count = 5;
    let mut secret_vec: Vec<u8> = std::iter::repeat_n(0u8, 32).collect();
    secret_vec
        .iter_mut()
        .enumerate()
        .take(21)
        .for_each(|(i, v)| *v = if i == 0 { 0xAA } else { (i * 5) as u8 });
    let secret: [u8; 32] = secret_vec.try_into().expect("32 bytes");

    let mut shares = split_secret(secret, threshold, shares_count).unwrap();

    // tamper with one share's first byte
    shares[1].1[0] ^= 0xFF;
    let result = combine_shares(&shares[0..threshold as usize], threshold);
    // Combining may succeed but should not equal original secret
    assert!(result.is_ok());
    assert_ne!(result.unwrap(), secret);
}
