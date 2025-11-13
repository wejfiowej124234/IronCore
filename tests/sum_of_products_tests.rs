//! tests/sum_of_products_tests.rs

use defi_hot_wallet::tools::sum_of_products;
use k256::{ProjectivePoint, Scalar}; // 纭繚瀵煎叆 Scalar

#[test]
fn sum_of_products_basic() {
    // 1*G + 2*(2*G) = G + 4G = 5G
    let one = Scalar::ONE;
    let two = Scalar::from(2u64);
    let scalars = vec![one, two];

    let g = ProjectivePoint::GENERATOR;
    let g2 = g * two;
    let points = vec![g, g2];

    let result = sum_of_products::sum_of_products(&scalars, &points);
    assert!(result.is_ok());
    let sum = result.unwrap();

    let expected = g * Scalar::from(5u64);
    assert_eq!(sum, expected);
}

#[test]
fn sum_of_products_empty_input() {
    let scalars: Vec<Scalar> = Vec::new();
    let points: Vec<ProjectivePoint> = Vec::new();

    let result = sum_of_products::sum_of_products(&scalars, &points);
    assert!(result.is_ok());
    let sum = result.unwrap();

    // An empty sum should result in the identity element (point at infinity).
    assert_eq!(sum, ProjectivePoint::IDENTITY);
}

#[test]
fn sum_of_products_mismatched_lengths() {
    let scalars = vec![Scalar::ONE];
    let points: Vec<ProjectivePoint> = Vec::new();

    let result = sum_of_products::sum_of_products(&scalars, &points);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Mismatched lengths of scalars and points");
}

#[test]
fn sum_of_products_large_input() {
    let scalars: Vec<Scalar> = (1..=100).map(|i| Scalar::from(i as u64)).collect();
    let points: Vec<ProjectivePoint> =
        (1..=100).map(|i| ProjectivePoint::GENERATOR * Scalar::from(i as u64)).collect();

    let result = sum_of_products::sum_of_products(&scalars, &points);
    assert!(result.is_ok());
    let sum = result.unwrap();

    // Expected: sum_{i=1 to 100} i * (i * G) = (sum_{i=1 to 100} i^2) * G
    let sum_of_squares: u64 = (1..=100).map(|i| i * i).sum();
    let expected = ProjectivePoint::GENERATOR * Scalar::from(sum_of_squares);
    assert_eq!(sum, expected);
}
