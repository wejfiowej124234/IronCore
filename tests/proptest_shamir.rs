use defi_hot_wallet::security::SecretVec;
use defi_hot_wallet::shamir::{combine_shares, split_secret};
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_random_secret(
        threshold in 1u8..5u8,
        total in 1u8..6u8,
        secret in proptest::collection::vec(any::<u8>(), 1..33)
    ) {
        // Require threshold <= total
        prop_assume!(threshold <= total);

        let secret_slice = secret.as_slice();
        let shares = split_secret(secret_slice, threshold, total).unwrap();
        // pick first `threshold` shares to reconstruct
        let subset: Vec<SecretVec> = shares.into_iter().take(threshold as usize).collect();
    let recovered = combine_shares(&subset).unwrap();
    prop_assert_eq!(recovered.as_slice(), secret.as_slice());
    }
}
