pub mod encryption_consistency;
pub mod hsm;
pub mod kdf;
pub mod multisig;
pub mod quantum;
pub mod secure_derivation;  // 🔐 Secure key derivation
pub mod shamir;
pub mod signature_utils;

pub use self::encryption_consistency::{
    get_global_statistics, init_global_validator, validate_global_consistency, EncryptionAlgorithm,
    EncryptionConsistencyValidator,
};
pub use self::hsm::HSMManager;
pub use self::kdf::KeyDerivation;
pub use self::multisig::MultiSignature;
pub use self::quantum::QuantumSafeEncryption;
// Fix: export shamir symbols from the crypto::shamir module (not from security::shamir)
pub use self::shamir::{combine_secret, combine_shares, split_secret};
