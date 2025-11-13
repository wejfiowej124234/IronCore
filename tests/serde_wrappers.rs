//! src/tools/serde_wrappers.rs
//!
//! Provides wrapper types with manual `serde` implementations for external
//! crate types that do not have native `serde` support.

use p256::elliptic_curve::sec1::FromEncodedPoint;
use p256::{AffinePoint, EncodedPoint, ProjectivePoint};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper around `k256::ProjectivePoint` to manually implement `serde`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectivePointWrapper(pub ProjectivePoint);

impl Serialize for ProjectivePointWrapper {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let encoded = EncodedPoint::from(self.0.to_affine());
        serializer.serialize_bytes(encoded.as_bytes())
    }
}

impl<'de> Deserialize<'de> for ProjectivePointWrapper {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        let encoded = EncodedPoint::from_bytes(&bytes).map_err(DeError::custom)?;
        let affine = Option::<AffinePoint>::from(AffinePoint::from_encoded_point(&encoded))
            .ok_or_else(|| DeError::custom("invalid encoded point"))?;
        Ok(Self(ProjectivePoint::from(affine)))
    }
}
