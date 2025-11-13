use anyhow::{anyhow, Result};
use elliptic_curve::group::{Group, GroupEncoding};
use serde::{Deserialize, Serialize};

/// Test struct with single scalar and point (for serialization tests).
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestStruct<G>
where
    G: Group + GroupEncoding,
{
    #[serde(with = "crate::tools::serdes::prime_field")]
    pub scalar: G::Scalar,
    #[serde(with = "crate::tools::serdes::group")]
    pub point: G,
}

/// Test struct with arrays (for serialization tests)
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestStructArray<G, const N: usize>
where
    G: Group + GroupEncoding,
{
    #[serde(with = "crate::tools::serdes::prime_field_array")]
    pub scalars: [G::Scalar; N],
    #[serde(with = "crate::tools::serdes::group_array")]
    pub points: [G; N],
}

/// Test struct with vectors (for serialization tests)
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestStructVec<G>
where
    G: Group + GroupEncoding,
{
    #[serde(with = "crate::tools::serdes::prime_field_vec")]
    pub scalars: Vec<G::Scalar>,
    #[serde(with = "crate::tools::serdes::group_vec")]
    pub points: Vec<G>,
}

/// Calculates the sum of products of scalars and points.
///
/// This is a naive implementation. In a real-world scenario,
/// this would be replaced by a more efficient algorithm like
/// Strauss's or Pippenger's algorithm.
pub fn sum_of_products<G>(scalars: &[G::Scalar], points: &[G]) -> Result<G>
where
    G: Group,
{
    if scalars.len() != points.len() {
        return Err(anyhow!("Mismatched lengths of scalars and points"));
    }

    Ok(scalars.iter().zip(points.iter()).map(|(s, p)| *p * *s).sum())
}
