use p256::{
    elliptic_curve::{
        group::{Group, GroupEncoding}, // GroupEncoding is needed for serdes::group
        Field,
    },
    ProjectivePoint, Scalar,
};
use serde::{Deserialize, Serialize};

// Use the serdes helpers from the elliptic-curve-tools crate
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
struct TestStruct {
    #[serde(with = "elliptic_curve_tools::serdes::prime_field")] // Use the hex-based serializer for scalars
    scalar: Scalar,
    #[serde(with = "elliptic_curve_tools::serdes::group")] // Use the hex-based serializer for group elements
    point: ProjectivePoint,
}

#[test]
fn p256_serialization_roundtrip() {
    let test_struct = TestStruct {
        scalar: <Scalar as Field>::ONE,
        point: ProjectivePoint::GENERATOR,
    };

    // JSON
    let json = serde_json::to_string(&test_struct).expect("json serialization failed");
    let from_json: TestStruct = serde_json::from_str(&json).expect("json deserialization failed");
    // The assertion should now pass as hex serialization is consistent.
    assert_eq!(test_struct, from_json);

    // Bincode
    let bincode = bincode::serialize(&test_struct).expect("bincode serialization failed");
    let from_bincode: TestStruct =
        bincode::deserialize(&bincode).expect("bincode deserialization failed");
    // The assertion should now pass as hex serialization is consistent.
    assert_eq!(test_struct, from_bincode);
}
