// tests/encryption_consistency_tests.rs
//! Tests for quantum encryption consistency validation

use defi_hot_wallet::crypto::{
    encryption_consistency::{
        get_global_statistics, get_global_validator, init_global_validator,
        validate_global_consistency, EncryptionAlgorithm, EncryptionConsistencyValidator,
    },
    QuantumSafeEncryption,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_encryption_consistency_validation() {
    // Initialize validator with quantum crypto
    let quantum_crypto = QuantumSafeEncryption::new().unwrap();
    init_global_validator(Some(quantum_crypto)).unwrap();

    // Create a wallet (this should register quantum-safe operations)
    // Note: We can't easily create a real wallet here without setting up storage,
    // so we'll manually register some operations for testing

    // Register some quantum-safe operations
    if let Ok(mut validator) = get_global_validator() {
        validator.register_operation(
            "test_quantum_encrypt",
            EncryptionAlgorithm::QuantumSafe,
            true,
            "test",
            1,
        );
        validator.register_operation(
            "test_quantum_decrypt",
            EncryptionAlgorithm::QuantumSafe,
            true,
            "test",
            2,
        );
    }

    // Register some traditional operations
    if let Ok(mut validator) = get_global_validator() {
        validator.register_operation(
            "test_aes_encrypt",
            EncryptionAlgorithm::Aes256Gcm,
            false,
            "test",
            3,
        );
        validator.register_operation(
            "test_aes_decrypt",
            EncryptionAlgorithm::Aes256Gcm,
            false,
            "test",
            4,
        );
    }

    // Validation should pass
    assert!(validate_global_consistency().is_ok());

    // Register a violating operation (non-quantum when quantum required)
    if let Ok(mut validator) = get_global_validator() {
        validator.register_operation(
            "test_violation",
            EncryptionAlgorithm::Aes256Gcm,
            true,
            "test",
            5,
        );
    }

    // Validation should now fail
    assert!(validate_global_consistency().is_err());
}

#[tokio::test]
#[serial]
async fn test_encryption_statistics() {
    // Initialize validator without quantum crypto
    init_global_validator(None).unwrap();

    // Register various operations
    if let Ok(mut validator) = get_global_validator() {
        validator.register_operation(
            "quantum_op",
            EncryptionAlgorithm::QuantumSafe,
            true,
            "test",
            1,
        );
        validator.register_operation("aes_op1", EncryptionAlgorithm::Aes256Gcm, false, "test", 2);
        validator.register_operation("aes_op2", EncryptionAlgorithm::Aes256Gcm, false, "test", 3);
        validator.register_operation("hkdf_op", EncryptionAlgorithm::Hkdf, true, "test", 4);
    }

    let stats = get_global_statistics().unwrap();

    assert_eq!(stats.quantum_safe_operations, 1);
    assert_eq!(stats.aes_operations, 2);
    assert_eq!(stats.hkdf_operations, 1);
    assert_eq!(stats.quantum_safe_required_operations, 2);
    assert_eq!(stats.total_operations(), 4);
    assert_eq!(stats.compliance_percentage(), 50.0);
}

#[tokio::test]
async fn test_quantum_crypto_validation() {
    let quantum_crypto = QuantumSafeEncryption::new().unwrap();
    let validator = EncryptionConsistencyValidator::new().with_quantum_crypto(quantum_crypto);

    // Should pass quantum crypto setup validation
    assert!(validator.validate_quantum_crypto_setup().is_ok());
}

#[test]
fn test_encryption_algorithm_enum() {
    // Test that all algorithm variants work
    assert_eq!(EncryptionAlgorithm::QuantumSafe as u8, 0);
    assert_eq!(EncryptionAlgorithm::Aes256Gcm as u8, 1);
    assert_eq!(EncryptionAlgorithm::Argon2 as u8, 2);
    assert_eq!(EncryptionAlgorithm::Pbkdf2 as u8, 3);
    assert_eq!(EncryptionAlgorithm::Scrypt as u8, 4);
    assert_eq!(EncryptionAlgorithm::Hkdf as u8, 5);
}
