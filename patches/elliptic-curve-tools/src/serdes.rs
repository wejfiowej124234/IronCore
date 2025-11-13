use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Generic passthrough serializer that forwards to the type's serde impl if it exists.
/// For types that implement Serialize, this will work directly.
pub fn serialize<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    v.serialize(serializer)
}

/// Generic passthrough deserializer that forwards to the type's serde impl.
pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer)
}

/// Modules expected by tests: prime_field, group, prime_field_array, group_vec,
/// and also aliases group_array and prime_field_vec.
pub mod prime_field {
    use super::*;
    pub fn serialize<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        super::serialize(v, serializer)
    }
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        super::deserialize(deserializer)
    }
}

pub mod group {
    use super::*;
    pub fn serialize<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        super::serialize(v, serializer)
    }
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        super::deserialize(deserializer)
    }
}

pub mod prime_field_array {
    use super::*;
    pub fn serialize<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        super::serialize(v, serializer)
    }
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        super::deserialize(deserializer)
    }
}

pub mod group_vec {
    use super::*;
    pub fn serialize<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        super::serialize(v, serializer)
    }
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        super::deserialize(deserializer)
    }
}

/// Aliases expected by tests that referenced different names.
pub mod group_array {
    pub use crate::serdes::group::{deserialize, serialize};
}
pub mod prime_field_vec {
    pub use crate::serdes::prime_field::{deserialize, serialize};
}
