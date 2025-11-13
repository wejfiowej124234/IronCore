use elliptic_curve::{group::GroupEncoding, Group, PrimeField};
use serde::{
    de::{Deserialize, Deserializer},
    ser::Serializer,
};

/// Serialize a PrimeField as hex string
pub mod prime_field {
    use super::*;

    pub fn serialize<F, S>(field: &F, serializer: S) -> Result<S::Ok, S::Error>
    where
        F: PrimeField,
        S: Serializer,
    {
        let bytes = field.to_repr();
        let hex = hex::encode(bytes.as_ref());
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, F, D>(deserializer: D) -> Result<F, D::Error>
    where
        F: PrimeField,
        D: Deserializer<'de>,
    {
        let hex = String::deserialize(deserializer)?;
        let bytes = hex::decode(&hex).map_err(serde::de::Error::custom)?;
        let mut repr = F::Repr::default();
        if bytes.len() != repr.as_ref().len() {
            return Err(serde::de::Error::custom("invalid field element length"));
        }
        repr.as_mut().copy_from_slice(&bytes);
        Option::from(F::from_repr(repr))
            .ok_or_else(|| serde::de::Error::custom("invalid field element"))
    }
}

/// Serialize a Group element as compressed hex
pub mod group {
    use super::*;

    pub fn serialize<G, S>(group: &G, serializer: S) -> Result<S::Ok, S::Error>
    where
        G: Group + GroupEncoding,
        S: Serializer,
    {
        let bytes = group.to_bytes();
        let hex = hex::encode(bytes.as_ref());
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, G, D>(deserializer: D) -> Result<G, D::Error>
    where
        G: Group + GroupEncoding,
        D: Deserializer<'de>,
    {
        let hex = String::deserialize(deserializer)?;
        let bytes = hex::decode(&hex).map_err(serde::de::Error::custom)?;
        let mut repr = G::Repr::default();
        let expected_len = repr.as_ref().len();
        if bytes.len() != expected_len {
            return Err(serde::de::Error::custom("invalid group element length"));
        }
        repr.as_mut().copy_from_slice(&bytes);

        Option::from(G::from_bytes(&repr))
            .ok_or_else(|| serde::de::Error::custom("invalid group element"))
    }
}

/// Serialize array of PrimeField
pub mod prime_field_array {
    use super::*;

    pub fn serialize<F, S, const N: usize>(
        fields: &[F; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        F: PrimeField,
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(N))?;
        for field in fields {
            let bytes = field.to_repr();
            let hex = hex::encode(bytes.as_ref());
            seq.serialize_element(&hex)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, F, D, const N: usize>(deserializer: D) -> Result<[F; N], D::Error>
    where
        F: PrimeField,
        D: Deserializer<'de>,
    {
        let vec = Vec::<String>::deserialize(deserializer)?;
        if vec.len() != N {
            return Err(serde::de::Error::custom(format!(
                "expected {} elements, got {}",
                N,
                vec.len()
            )));
        }

        let mut result = Vec::with_capacity(N);
        for hex in vec {
            let bytes = hex::decode(&hex).map_err(serde::de::Error::custom)?;
            let mut repr = F::Repr::default();
            if bytes.len() != repr.as_ref().len() {
                return Err(serde::de::Error::custom("invalid field element length"));
            }
            repr.as_mut().copy_from_slice(&bytes);
            let field = Option::from(F::from_repr(repr))
                .ok_or_else(|| serde::de::Error::custom("invalid field element"))?;
            result.push(field);
        }

        result.try_into().map_err(|_| serde::de::Error::custom("array conversion failed"))
    }
}

/// Serialize array of Group elements
pub mod group_array {
    use super::*;

    pub fn serialize<G, S, const N: usize>(
        groups: &[G; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        G: Group + GroupEncoding,
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(N))?;
        for group in groups {
            let bytes = group.to_bytes();
            let hex = hex::encode(bytes.as_ref());
            seq.serialize_element(&hex)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, G, D, const N: usize>(deserializer: D) -> Result<[G; N], D::Error>
    where
        G: Group + GroupEncoding,
        D: Deserializer<'de>,
    {
        let vec = Vec::<String>::deserialize(deserializer)?;
        if vec.len() != N {
            return Err(serde::de::Error::custom(format!(
                "expected {} elements, got {}",
                N,
                vec.len()
            )));
        }

        let mut result = Vec::with_capacity(N);
        for hex in vec {
            let bytes = hex::decode(&hex).map_err(serde::de::Error::custom)?;
            let mut repr = G::Repr::default();
            let expected_len = repr.as_ref().len();
            if bytes.len() != expected_len {
                return Err(serde::de::Error::custom("invalid group element length"));
            }
            repr.as_mut().copy_from_slice(&bytes);
            let group = Option::from(G::from_bytes(&repr))
                .ok_or_else(|| serde::de::Error::custom("invalid group element"))?;
            result.push(group);
        }

        result.try_into().map_err(|_| serde::de::Error::custom("array conversion failed"))
    }
}

/// Serialize Vec of PrimeField
pub mod prime_field_vec {
    use super::*;

    pub fn serialize<F, S>(fields: &Vec<F>, serializer: S) -> Result<S::Ok, S::Error>
    where
        F: PrimeField,
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(fields.len()))?;
        for field in fields {
            let bytes = field.to_repr();
            let hex = hex::encode(bytes.as_ref());
            seq.serialize_element(&hex)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, F, D>(deserializer: D) -> Result<Vec<F>, D::Error>
    where
        F: PrimeField,
        D: Deserializer<'de>,
    {
        let vec = Vec::<String>::deserialize(deserializer)?;
        let mut result = Vec::with_capacity(vec.len());

        for hex in vec {
            let bytes = hex::decode(&hex).map_err(serde::de::Error::custom)?;
            let mut repr = F::Repr::default();
            if bytes.len() != repr.as_ref().len() {
                return Err(serde::de::Error::custom("invalid field element length"));
            }
            repr.as_mut().copy_from_slice(&bytes);
            let field = Option::from(F::from_repr(repr))
                .ok_or_else(|| serde::de::Error::custom("invalid field element"))?;
            result.push(field);
        }

        Ok(result)
    }
}

/// Serialize Vec of Group elements
pub mod group_vec {
    use super::*;

    pub fn serialize<G, S>(groups: &Vec<G>, serializer: S) -> Result<S::Ok, S::Error>
    where
        G: Group + GroupEncoding,
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(groups.len()))?;
        for group in groups {
            let bytes = group.to_bytes();
            let hex = hex::encode(bytes.as_ref());
            seq.serialize_element(&hex)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, G, D>(deserializer: D) -> Result<Vec<G>, D::Error>
    where
        G: Group + GroupEncoding,
        D: Deserializer<'de>,
    {
        let vec = Vec::<String>::deserialize(deserializer)?;
        let mut result = Vec::with_capacity(vec.len());

        for hex in vec {
            let bytes = hex::decode(&hex).map_err(serde::de::Error::custom)?;
            let mut repr = G::Repr::default();
            let expected_len = repr.as_ref().len();
            if bytes.len() != expected_len {
                return Err(serde::de::Error::custom("invalid group element length"));
            }
            repr.as_mut().copy_from_slice(&bytes);
            let group = Option::from(G::from_bytes(&repr))
                .ok_or_else(|| serde::de::Error::custom("invalid group element"))?;
            result.push(group);
        }

        Ok(result)
    }
}
