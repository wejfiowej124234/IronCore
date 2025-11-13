use defi_hot_wallet::tools::sum_of_products::{TestStruct, TestStructArray, TestStructVec};
use elliptic_curve::Field;

type K256Point = k256::ProjectivePoint;
// Helper type alias for k256 with serde support
type K256Scalar = k256::Scalar;

#[test]
fn basic_k256_serialization() {
    let test_struct =
        TestStruct { scalar: <K256Scalar as Field>::ONE, point: K256Point::GENERATOR };

    // JSON test
    let json = serde_json::to_string(&test_struct).expect("JSON serialization failed");
    let from_json: TestStruct<K256Point> =
        serde_json::from_str(&json).expect("JSON deserialization failed");
    assert_eq!(test_struct, from_json);

    // Bincode test
    let bincode = bincode::serialize(&test_struct).expect("Bincode serialization failed");
    let from_bincode: TestStruct<K256Point> =
        bincode::deserialize(&bincode).expect("Bincode deserialization failed");
    assert_eq!(test_struct, from_bincode);
}

#[test]
fn boundary_scalars() {
    let scalars = vec![K256Scalar::ZERO, <K256Scalar as Field>::ONE, K256Scalar::from(u64::MAX)];

    for scalar in scalars {
        let test_struct = TestStruct { scalar, point: K256Point::GENERATOR };

        let json = serde_json::to_string(&test_struct).unwrap();
        let from_json: TestStruct<K256Point> = serde_json::from_str(&json).unwrap();
        assert_eq!(test_struct, from_json);
    }
}

#[test]
fn identity_and_random_points() {
    let identity = TestStruct { scalar: <K256Scalar as Field>::ONE, point: K256Point::IDENTITY };

    let random = TestStruct {
        scalar: <K256Scalar as Field>::ONE,
        point: K256Point::GENERATOR * K256Scalar::from(42u64),
    };

    for test_struct in [identity, random] {
        let json = serde_json::to_string(&test_struct).unwrap();
        let from_json: TestStruct<K256Point> = serde_json::from_str(&json).unwrap();
        assert_eq!(test_struct, from_json);
    }
}

#[test]
fn array_structs() {
    let array_struct = TestStructArray::<K256Point, 2> {
        scalars: [<K256Scalar as Field>::ONE; 2],
        points: [K256Point::GENERATOR; 2],
    };

    let json = serde_json::to_string(&array_struct).unwrap();
    let from_json: TestStructArray<K256Point, 2> = serde_json::from_str(&json).unwrap();
    assert_eq!(array_struct, from_json);
}

#[test]
fn vec_structs() {
    let vec_struct = TestStructVec {
        scalars: vec![<K256Scalar as Field>::ONE; 10],
        points: vec![K256Point::GENERATOR; 10],
    };

    let bincode = bincode::serialize(&vec_struct).unwrap();
    let from_bincode: TestStructVec<K256Point> = bincode::deserialize(&bincode).unwrap();
    assert_eq!(vec_struct, from_bincode);
}

#[test]
fn empty_and_large_structs() {
    // Empty array
    let empty_array = TestStructArray::<K256Point, 0> { scalars: [], points: [] };
    let json = serde_json::to_string(&empty_array).unwrap();
    let from_json: TestStructArray<K256Point, 0> = serde_json::from_str(&json).unwrap();
    assert_eq!(empty_array, from_json);

    // Large vec
    let large_vec = TestStructVec {
        scalars: vec![<K256Scalar as Field>::ONE; 1000],
        points: vec![K256Point::GENERATOR; 1000],
    };
    let bincode = bincode::serialize(&large_vec).unwrap();
    let from_bincode: TestStructVec<K256Point> = bincode::deserialize(&bincode).unwrap();
    assert_eq!(large_vec, from_bincode);
}

#[test]
fn cross_format_consistency() {
    let test_struct =
        TestStruct { scalar: <K256Scalar as Field>::ONE, point: K256Point::GENERATOR };

    let json = serde_json::to_string(&test_struct).unwrap();
    let bincode = bincode::serialize(&test_struct).unwrap();
    let yaml = serde_yaml::to_string(&test_struct).unwrap();

    let from_json: TestStruct<K256Point> = serde_json::from_str(&json).unwrap();
    let from_bincode: TestStruct<K256Point> = bincode::deserialize(&bincode).unwrap();
    let from_yaml: TestStruct<K256Point> = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(test_struct, from_json);
    assert_eq!(test_struct, from_bincode);
    assert_eq!(test_struct, from_yaml);
}

#[test]
fn error_cases() {
    // Invalid JSON scalar
    let invalid_hex = r#""gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg""#;
    let res: Result<K256Scalar, _> = serde_json::from_str(invalid_hex);
    assert!(res.is_err());

    // Invalid JSON struct
    let invalid_json = r#"{"x": "invalid", "p": "invalid"}"#;
    let res: Result<TestStruct<K256Point>, _> = serde_json::from_str(invalid_json);
    assert!(res.is_err());

    // Invalid bincode
    let invalid_bincode = vec![0xFF; 10];
    let res: Result<TestStruct<K256Point>, _> = bincode::deserialize(&invalid_bincode);
    assert!(res.is_err());
}

#[test]
fn postcard_format() {
    let test_struct =
        TestStruct { scalar: <K256Scalar as Field>::ONE, point: K256Point::GENERATOR };

    let res = postcard::to_stdvec(&test_struct);
    assert!(res.is_ok());
    let output = res.unwrap();

    let from_postcard: TestStruct<K256Point> = postcard::from_bytes(&output).unwrap();
    assert_eq!(test_struct, from_postcard);
}

#[test]
fn cbor_format() {
    let test_struct =
        TestStruct { scalar: <K256Scalar as Field>::ONE, point: K256Point::GENERATOR };

    let mut cbor = Vec::new();
    ciborium::into_writer(&test_struct, &mut cbor).unwrap();
    let from_cbor: TestStruct<K256Point> = ciborium::from_reader(cbor.as_slice()).unwrap();
    assert_eq!(test_struct, from_cbor);
}

#[cfg(all(feature = "std", not(miri)))]
#[test]
fn concurrent_operations() {
    use serde_json;
    use std::thread;

    let json = serde_json::to_string(&TestStruct {
        scalar: <K256Scalar as Field>::ONE,
        point: K256Point::GENERATOR,
    })
    .unwrap();

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let json = json.clone();
            thread::spawn(move || {
                let res: Result<TestStruct<K256Point>, _> = serde_json::from_str(&json);
                assert!(res.is_ok());
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn performance_test() {
    let large_vec = TestStructVec {
        scalars: vec![<K256Scalar as Field>::ONE; 10000],
        points: vec![K256Point::GENERATOR; 10000],
    };

    let start = std::time::Instant::now();
    let bincode = bincode::serialize(&large_vec).unwrap();
    let serialize_duration = start.elapsed();
    println!("Serialization of 10k points took: {:?}", serialize_duration);

    let start = std::time::Instant::now();
    let _: TestStructVec<K256Point> = bincode::deserialize(&bincode).unwrap();
    let deserialize_duration = start.elapsed();

    println!("Deserialization of 10k points took: {:?}", deserialize_duration);
}
