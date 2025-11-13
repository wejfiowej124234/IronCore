// src/crypto/encryption_consistency.rs
//! Quantum Encryption Consistency Validator
//!
//! This module validates that all cryptographic operations consistently use
//! quantum-resistant algorithms when quantum-safe mode is enabled.

use crate::core::errors::WalletError;
use crate::crypto::quantum::QuantumSafeEncryption;
// HashMap previously imported but not required here; remove to satisfy lints.
use tracing::{debug, info, warn};

/// Encryption algorithm types with explicit numeric discriminants for test consistency
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    /// Quantum-safe encryption (Kyber + AES-GCM)
    QuantumSafe = 0,
    /// AES-256-GCM (not quantum-resistant)
    Aes256Gcm = 1,
    /// Argon2 key derivation (not quantum-resistant)
    Argon2 = 2,
    /// PBKDF2 key derivation (not quantum-resistant)
    Pbkdf2 = 3,
    /// Scrypt key derivation (not quantum-resistant)
    Scrypt = 4,
    /// HKDF key derivation (quantum-resistant if used with quantum-safe inputs)
    Hkdf = 5,
}

/// Encryption operation context
#[derive(Debug, Clone)]
pub struct EncryptionContext {
    pub operation: String,
    pub algorithm: EncryptionAlgorithm,
    pub quantum_safe_required: bool,
    pub module: String,
    pub line: u32,
}

/// Quantum encryption consistency validator
pub struct EncryptionConsistencyValidator {
    contexts: Vec<EncryptionContext>,
    quantum_crypto: Option<QuantumSafeEncryption>,
}

impl EncryptionConsistencyValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self { contexts: Vec::new(), quantum_crypto: None }
    }

    /// Set the quantum crypto instance for validation
    pub fn with_quantum_crypto(mut self, crypto: QuantumSafeEncryption) -> Self {
        self.quantum_crypto = Some(crypto);
        self
    }

    /// Register an encryption operation for validation
    pub fn register_operation(
        &mut self,
        operation: impl Into<String>,
        algorithm: EncryptionAlgorithm,
        quantum_safe_required: bool,
        module: impl Into<String>,
        line: u32,
    ) {
        let context = EncryptionContext {
            operation: operation.into(),
            algorithm,
            quantum_safe_required,
            module: module.into(),
            line,
        };

        debug!(
            "Registered encryption operation: {} ({:?}) in {}:{} (quantum_safe: {})",
            context.operation,
            context.algorithm,
            context.module,
            context.line,
            context.quantum_safe_required
        );

        self.contexts.push(context);
    }

    /// Validate all registered operations for quantum encryption consistency
    pub fn validate_consistency(&self) -> Result<(), WalletError> {
        info!(
            "Validating quantum encryption consistency across {} operations",
            self.contexts.len()
        );

        let mut violations = Vec::new();

        for context in &self.contexts {
            if let Some(violation) = self.check_operation_consistency(context) {
                violations.push(violation);
            }
        }

        if !violations.is_empty() {
            let error_msg = format!(
                "Quantum encryption consistency violations found:\n{}",
                violations.iter().map(|v| format!("  - {}", v)).collect::<Vec<_>>().join("\n")
            );
            warn!("{}", error_msg);
            return Err(WalletError::CryptoError(error_msg));
        }

        info!("✅ All encryption operations are quantum-consistent");
        Ok(())
    }

    /// Check if a single operation is consistent with quantum safety requirements
    fn check_operation_consistency(&self, context: &EncryptionContext) -> Option<String> {
        match (&context.algorithm, context.quantum_safe_required) {
            // Quantum-safe operations are always allowed
            (EncryptionAlgorithm::QuantumSafe, _) => None,

            // Non-quantum-safe operations are only allowed when quantum safety is not required
            (EncryptionAlgorithm::Aes256Gcm, false) => None,
            (EncryptionAlgorithm::Argon2, false) => None,
            (EncryptionAlgorithm::Pbkdf2, false) => None,
            (EncryptionAlgorithm::Scrypt, false) => None,

            // HKDF is quantum-resistant if used with quantum-safe inputs
            (EncryptionAlgorithm::Hkdf, _) => None,

            // Any non-quantum-safe operation when quantum safety is required is a violation
            (algorithm, true) => Some(format!(
                "Operation '{}' in {}:{} uses non-quantum-safe algorithm {:?} but quantum safety is required",
                context.operation, context.module, context.line, algorithm
            )),
        }
    }

    /// Get statistics about registered operations
    pub fn get_statistics(&self) -> EncryptionStatistics {
        let mut stats = EncryptionStatistics::default();

        for context in &self.contexts {
            match context.algorithm {
                EncryptionAlgorithm::QuantumSafe => stats.quantum_safe_operations += 1,
                EncryptionAlgorithm::Aes256Gcm => stats.aes_operations += 1,
                EncryptionAlgorithm::Argon2 => stats.argon2_operations += 1,
                EncryptionAlgorithm::Pbkdf2 => stats.pbkdf2_operations += 1,
                EncryptionAlgorithm::Scrypt => stats.scrypt_operations += 1,
                EncryptionAlgorithm::Hkdf => stats.hkdf_operations += 1,
            }

            if context.quantum_safe_required {
                stats.quantum_safe_required_operations += 1;
            }
        }

        stats
    }

    /// Validate that quantum crypto instance is properly configured
    pub fn validate_quantum_crypto_setup(&self) -> Result<(), WalletError> {
        if self.quantum_crypto.is_none() {
            return Err(WalletError::CryptoError(
                "Quantum crypto instance not configured for validation".to_string(),
            ));
        }

        // Test basic quantum crypto functionality
        let crypto = self.quantum_crypto.as_ref().unwrap();
        let test_data = b"test_data_for_validation";
        let test_key = &[0u8; 32];

        let encrypted = crypto.encrypt(test_data, test_key).map_err(|e| {
            WalletError::CryptoError(format!("Quantum crypto encryption test failed: {}", e))
        })?;

        let decrypted = crypto.decrypt(&encrypted, test_key).map_err(|e| {
            WalletError::CryptoError(format!("Quantum crypto decryption test failed: {}", e))
        })?;

        if decrypted.as_slice() != test_data {
            return Err(WalletError::CryptoError(
                "Quantum crypto roundtrip test failed: decrypted data doesn't match original"
                    .to_string(),
            ));
        }

        debug!("✅ Quantum crypto setup validation passed");
        Ok(())
    }
}

impl Default for EncryptionConsistencyValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about encryption operations
#[derive(Debug, Default, Clone)]
pub struct EncryptionStatistics {
    pub quantum_safe_operations: usize,
    pub aes_operations: usize,
    pub argon2_operations: usize,
    pub pbkdf2_operations: usize,
    pub scrypt_operations: usize,
    pub hkdf_operations: usize,
    pub quantum_safe_required_operations: usize,
}

impl EncryptionStatistics {
    /// Get total number of operations
    pub fn total_operations(&self) -> usize {
        self.quantum_safe_operations
            + self.aes_operations
            + self.argon2_operations
            + self.pbkdf2_operations
            + self.scrypt_operations
            + self.hkdf_operations
    }

    /// Get compliance percentage (quantum-safe operations / quantum-safe required operations)
    pub fn compliance_percentage(&self) -> f64 {
        if self.quantum_safe_required_operations == 0 {
            100.0
        } else {
            (self.quantum_safe_operations as f64 / self.quantum_safe_required_operations as f64)
                * 100.0
        }
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
/// Global encryption consistency validator instance (thread-safe)
use std::sync::{Mutex, OnceLock};

static GLOBAL_VALIDATOR: OnceLock<Mutex<EncryptionConsistencyValidator>> = OnceLock::new();

// Runtime toggle to control whether registration macros actually record operations.
// Default: disabled so unrelated tests don't race on the global validator.
static REGISTRATION_ENABLED: OnceLock<AtomicBool> = OnceLock::new();

fn registration_flag() -> &'static AtomicBool {
    REGISTRATION_ENABLED.get_or_init(|| AtomicBool::new(false))
}

/// Returns true if encryption operations registration is enabled.
pub fn should_register_encryption_operations() -> bool {
    registration_flag().load(Ordering::Relaxed)
}

/// Initialize the global encryption consistency validator
pub fn init_global_validator(
    quantum_crypto: Option<QuantumSafeEncryption>,
) -> Result<(), WalletError> {
    let m = GLOBAL_VALIDATOR.get_or_init(|| Mutex::new(EncryptionConsistencyValidator::new()));
    let mut guard = m
        .lock()
        .map_err(|_| WalletError::CryptoError("Global validator lock poisoned".to_string()))?;
    *guard = EncryptionConsistencyValidator::new();
    if let Some(crypto) = quantum_crypto {
        guard.quantum_crypto = Some(crypto);
    }
    // Enable registration once explicitly initialized by the caller (tests or app startup).
    registration_flag().store(true, Ordering::Relaxed);
    info!("Global encryption consistency validator initialized");
    Ok(())
}

/// Get a guard to the global validator (for registration)
pub fn get_global_validator(
) -> Result<std::sync::MutexGuard<'static, EncryptionConsistencyValidator>, WalletError> {
    let m = GLOBAL_VALIDATOR.get_or_init(|| Mutex::new(EncryptionConsistencyValidator::new()));
    m.lock().map_err(|_| {
        WalletError::CryptoError("Global encryption validator lock poisoned".to_string())
    })
}

/// Run global consistency validation
pub fn validate_global_consistency() -> Result<(), WalletError> {
    // First validate consistency while holding the lock briefly
    {
        let validator = get_global_validator()?;
        validator.validate_consistency()?;
    }

    // Extract the optional quantum crypto instance without holding it during the crypto round-trip
    let maybe_crypto = {
        let mut guard = get_global_validator()?;
        guard.quantum_crypto.take()
    };

    if let Some(crypto) = maybe_crypto {
        // Perform quantum crypto validation outside the global lock
        let tmp_validator = EncryptionConsistencyValidator::new().with_quantum_crypto(crypto);
        let res = tmp_validator.validate_quantum_crypto_setup();

        // Restore the quantum crypto instance back into the global validator
        if let Ok(mut guard) = get_global_validator() {
            guard.quantum_crypto = tmp_validator.quantum_crypto;
        }

        return res;
    }

    Err(WalletError::CryptoError(
        "Quantum crypto instance not configured for validation".to_string(),
    ))
}

/// Get global statistics
pub fn get_global_statistics() -> Result<EncryptionStatistics, WalletError> {
    let validator = get_global_validator()?;
    Ok(validator.get_statistics())
}

/// Macro to register an encryption operation for consistency validation
#[macro_export]
macro_rules! register_encryption_operation {
    ($operation:expr, $algorithm:expr, $quantum_safe:expr) => {
        if $crate::crypto::encryption_consistency::should_register_encryption_operations() {
            if let Ok(mut validator) =
                $crate::crypto::encryption_consistency::get_global_validator()
            {
                validator.register_operation(
                    $operation,
                    $algorithm,
                    $quantum_safe,
                    module_path!(),
                    line!(),
                );
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_consistency_validator() {
        let mut validator = EncryptionConsistencyValidator::new();

        // Register some operations
        validator.register_operation(
            "wallet_create",
            EncryptionAlgorithm::QuantumSafe,
            true,
            "test",
            1,
        );
        validator.register_operation("key_derive", EncryptionAlgorithm::Argon2, false, "test", 2);
        validator.register_operation(
            "data_encrypt",
            EncryptionAlgorithm::Aes256Gcm,
            false,
            "test",
            3,
        );

        // Should pass validation
        assert!(validator.validate_consistency().is_ok());

        // Register a violating operation
        validator.register_operation(
            "bad_encrypt",
            EncryptionAlgorithm::Aes256Gcm,
            true,
            "test",
            4,
        );

        // Should fail validation
        assert!(validator.validate_consistency().is_err());
    }

    #[test]
    fn test_statistics() {
        let mut validator = EncryptionConsistencyValidator::new();

        validator.register_operation(
            "quantum_op",
            EncryptionAlgorithm::QuantumSafe,
            true,
            "test",
            1,
        );
        validator.register_operation("aes_op", EncryptionAlgorithm::Aes256Gcm, false, "test", 2);
        validator.register_operation("hkdf_op", EncryptionAlgorithm::Hkdf, true, "test", 3);

        let stats = validator.get_statistics();
        assert_eq!(stats.quantum_safe_operations, 1);
        assert_eq!(stats.aes_operations, 1);
        assert_eq!(stats.hkdf_operations, 1);
        assert_eq!(stats.quantum_safe_required_operations, 2);
        assert_eq!(stats.total_operations(), 3);
        assert_eq!(stats.compliance_percentage(), 50.0);
    }

    #[test]
    fn test_quantum_crypto_validation() {
        let crypto = QuantumSafeEncryption::new().unwrap();
        let validator = EncryptionConsistencyValidator::new().with_quantum_crypto(crypto);

        // Should pass quantum crypto setup validation
        assert!(validator.validate_quantum_crypto_setup().is_ok());
    }
}
