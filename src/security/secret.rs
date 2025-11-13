//! Small helpers and aliases for secret buffers that must be zeroized on drop.
use zeroize::Zeroizing;

/// Common alias for secret byte buffers which will be zeroed when dropped.
pub type SecretVec = Zeroizing<Vec<u8>>;

/// Convert a Vec<u8> into a `SecretVec` which will be zeroized on drop.
pub fn vec_to_secret(v: Vec<u8>) -> SecretVec {
    Zeroizing::new(v)
}
