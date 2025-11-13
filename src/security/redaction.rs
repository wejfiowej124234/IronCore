// Simple helpers to avoid accidental printing of secrets in logs/tests.
use std::env;

/// Redact a text body unless DEV_PRINT_SECRETS=1 is set in the environment.
/// By default this returns a short placeholder containing only the length.
pub fn redact_body(s: &str) -> String {
    if env::var("DEV_PRINT_SECRETS").ok().as_deref() == Some("1") {
        // Developer explicitly allowed printing secrets
        return s.to_string();
    }
    format!("<redacted len={}>", s.len())
}

/// Redact hex-serializable bytes unless DEV_PRINT_SECRETS=1 is set.
pub fn redact_hex_bytes(bytes: &[u8]) -> String {
    if env::var("DEV_PRINT_SECRETS").ok().as_deref() == Some("1") {
        return format!("0x{}", hex::encode(bytes));
    }
    format!("<redacted hex len={}>", bytes.len())
}
