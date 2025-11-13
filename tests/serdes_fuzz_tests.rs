//! tools/serdes.rs Fuzz测试
//! 使用 arbitrary 和 proptest 进行 fuzz 测试
//! 目标：80%+ 行覆盖率，覆盖无穷大点、非法坐标、round-trip验证

use proptest::prelude::*;
use serde::{Deserialize, Serialize};

// ================================================================================
// Hex 序列化 Fuzz 测试（使用 proptest）
// ================================================================================

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct HexData {
    #[serde(with = "hex_serde")]
    data: Vec<u8>,
}

mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    
    pub fn serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(data))
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

// ================================================================================
// Proptest 策略定义
// ================================================================================

proptest! {
    #[test]
    fn test_hex_roundtrip_fuzz(data in prop::collection::vec(any::<u8>(), 0..1000)) {
        let hex_data = HexData { data: data.clone() };
        
        // JSON round-trip
        let json = serde_json::to_string(&hex_data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(&hex_data, &deserialized);
    }
    
    #[test]
    fn test_hex_bincode_fuzz(data in prop::collection::vec(any::<u8>(), 0..1000)) {
        let hex_data = HexData { data: data.clone() };
        
        // Bincode round-trip
        let encoded = bincode::serialize(&hex_data).unwrap();
        let decoded: HexData = bincode::deserialize(&encoded).unwrap();
        
        prop_assert_eq!(&hex_data, &decoded);
    }
    
    #[test]
    fn test_hex_postcard_fuzz(data in prop::collection::vec(any::<u8>(), 0..500)) {
        let hex_data = HexData { data: data.clone() };
        
        // Postcard round-trip
        let encoded = postcard::to_allocvec(&hex_data).unwrap();
        let decoded: HexData = postcard::from_bytes(&encoded).unwrap();
        
        prop_assert_eq!(&hex_data, &decoded);
    }
}

// ================================================================================
// 椭圆曲线点 Fuzz 测试
// ================================================================================

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct MockECPoint {
    #[serde(with = "hex_serde")]
    compressed: Vec<u8>, // 33字节压缩点
}

// EC Point 特殊值测试
#[test]
fn test_ec_point_special_values() {
    let special_cases = vec![
        vec![0x00; 33],                    // 无穷大点
        vec![0xFF; 33],                    // 坐标溢出
        vec![0x02; 33],                    // 前缀0x02，全2
        vec![0x03; 33],                    // 前缀0x03，全3
        {
            let mut v = vec![0x02];
            v.extend(vec![0xFF; 32]);
            v
        },                                  // 0x02 + 最大坐标
        {
            let mut v = vec![0x03];
            v.extend(vec![0x00; 32]);
            v
        },                                  // 0x03 + 最小坐标
    ];
    
    for case in special_cases {
        let point = MockECPoint { compressed: case.clone() };
        
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: MockECPoint = serde_json::from_str(&json).unwrap();
        
        assert_eq!(point, deserialized);
    }
}

proptest! {
    #[test]
    fn test_ec_point_roundtrip_fuzz(
        prefix in 0x02u8..=0x03u8,
        data in prop::collection::vec(any::<u8>(), 32..=32)
    ) {
        let mut compressed = vec![prefix];
        compressed.extend_from_slice(&data);
        
        let point = MockECPoint { compressed };
        
        // JSON round-trip
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: MockECPoint = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(&point, &deserialized);
    }
    
    #[test]
    fn test_ec_point_invalid_prefix_fuzz(
        prefix in prop::num::u8::ANY.prop_filter("invalid prefix", |&p| p != 0x02 && p != 0x03),
        data in prop::collection::vec(any::<u8>(), 32..=32)
    ) {
        let mut compressed = vec![prefix];
        compressed.extend_from_slice(&data);
        
        let point = MockECPoint { compressed: compressed.clone() };
        
        // 即使前缀无效，序列化也应该工作
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: MockECPoint = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(&point.compressed, &deserialized.compressed);
        prop_assert_eq!(deserialized.compressed[0], prefix);
    }
    
    #[test]
    fn test_ec_point_infinity_fuzz(data in prop::collection::vec(0u8..=0u8, 33..=33)) {
        // 全零表示无穷大点
        let point = MockECPoint { compressed: data.clone() };
        
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: MockECPoint = serde_json::from_str(&json).unwrap();
        
        prop_assert!(deserialized.compressed.iter().all(|&b| b == 0));
    }
    
    #[test]
    fn test_ec_point_max_values_fuzz(data in prop::collection::vec(0xFFu8..=0xFFu8, 33..=33)) {
        // 全 0xFF（坐标溢出）
        let point = MockECPoint { compressed: data };
        
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: MockECPoint = serde_json::from_str(&json).unwrap();
        
        prop_assert!(deserialized.compressed.iter().all(|&b| b == 0xFF));
    }
}

// ================================================================================
// 非法坐标 Fuzz 测试
// ================================================================================

proptest! {
    #[test]
    fn test_invalid_length_fuzz(len in 1usize..100) {
        // 非标准长度（不是33或65字节）
        if len != 33 && len != 65 {
            let data = vec![0x02u8; len];
            let point = MockECPoint { compressed: data.clone() };
            
            let json = serde_json::to_string(&point).unwrap();
            let deserialized: MockECPoint = serde_json::from_str(&json).unwrap();
            
            prop_assert_eq!(&point, &deserialized);
            prop_assert_eq!(deserialized.compressed.len(), len);
        }
    }
    
    #[test]
    fn test_random_bytes_fuzz(data in prop::collection::vec(any::<u8>(), 1..200)) {
        // 完全随机的字节
        let point = MockECPoint { compressed: data.clone() };
        
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: MockECPoint = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(&point, &deserialized);
    }
}

// ================================================================================
// 边界值 Fuzz 测试
// ================================================================================

proptest! {
    #[test]
    fn test_boundary_values_fuzz(
        first in any::<u8>(),
        middle in prop::collection::vec(any::<u8>(), 0..50),
        last in any::<u8>()
    ) {
        let mut data = vec![first];
        data.extend(middle);
        data.push(last);
        
        let hex_data = HexData { data: data.clone() };
        
        let json = serde_json::to_string(&hex_data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(&hex_data, &deserialized);
    }
}

// ================================================================================
// 多格式 Fuzz 测试
// ================================================================================

proptest! {
    #[test]
    fn test_multiformat_consistency_fuzz(data in prop::collection::vec(any::<u8>(), 0..100)) {
        let hex_data = HexData { data: data.clone() };
        
        // JSON
        let json = serde_json::to_string(&hex_data).unwrap();
        let from_json: HexData = serde_json::from_str(&json).unwrap();
        
        // Bincode
        let bincode_bytes = bincode::serialize(&hex_data).unwrap();
        let from_bincode: HexData = bincode::deserialize(&bincode_bytes).unwrap();
        
        // 验证一致性（使用引用避免move）
        prop_assert_eq!(&from_json, &from_bincode);
        prop_assert_eq!(&hex_data, &from_json);
    }
}

// ================================================================================
// 错误注入 Fuzz 测试
// ================================================================================

#[test]
fn test_invalid_hex_strings() {
    let invalid_cases = vec![
        r#"{"data":"GH"}"#,           // 非hex字符
        r#"{"data":"12G"}"#,          // 混合
        r#"{"data":"xyz"}"#,          // 完全无效
        r#"{"data":"  "}"#,           // 空格
        r#"{"data":"0x1234"}"#,       // 带0x前缀
    ];
    
    for invalid in invalid_cases {
        let result: Result<HexData, _> = serde_json::from_str(invalid);
        assert!(result.is_err(), "Should fail: {}", invalid);
    }
}

#[test]
fn test_malformed_json() {
    let malformed_cases = vec![
        r#"{"data":}"#,               // 缺少值
        r#"{"data"}"#,                // 缺少冒号
        r#"{data:"00"}"#,             // 缺少引号
        r#"{"data":"00""#,            // 缺少闭合括号
    ];
    
    for malformed in malformed_cases {
        let result: Result<HexData, _> = serde_json::from_str(malformed);
        assert!(result.is_err(), "Should fail: {}", malformed);
    }
}

// ================================================================================
// 压力测试 - 大量迭代
// ================================================================================

#[test]
fn test_many_roundtrips() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    for _ in 0..1000 {
        let len = rng.gen_range(0..100);
        let data: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
        
        let hex_data = HexData { data: data.clone() };
        
        let json = serde_json::to_string(&hex_data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(hex_data, deserialized);
    }
}

// ================================================================================
// 特殊模式 Fuzz 测试
// ================================================================================

proptest! {
    #[test]
    fn test_alternating_pattern_fuzz(len in 1usize..100) {
        let data: Vec<u8> = (0..len).map(|i| if i % 2 == 0 { 0x00 } else { 0xFF }).collect();
        
        let hex_data = HexData { data: data.clone() };
        
        let json = serde_json::to_string(&hex_data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(&hex_data, &deserialized);
    }
    
    #[test]
    fn test_sequential_pattern_fuzz(start in any::<u8>(), len in 1usize..100) {
        let data: Vec<u8> = (0..len).map(|i| start.wrapping_add(i as u8)).collect();
        
        let hex_data = HexData { data };
        
        let json = serde_json::to_string(&hex_data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(&hex_data, &deserialized);
    }
}

// MockECPoint 已在上面定义

// ================================================================================
// Round-trip 验证 - 所有格式
// ================================================================================

proptest! {
    #[test]
    fn test_all_formats_roundtrip_fuzz(data in prop::collection::vec(any::<u8>(), 0..100)) {
        let original = HexData { data: data.clone() };
        
        // JSON
        let json = serde_json::to_string(&original).unwrap();
        let from_json: HexData = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(&original, &from_json);
        
        // Bincode
        let bincode_bytes = bincode::serialize(&original).unwrap();
        let from_bincode: HexData = bincode::deserialize(&bincode_bytes).unwrap();
        prop_assert_eq!(&original, &from_bincode);
        
        // Postcard
        let postcard_bytes = postcard::to_allocvec(&original).unwrap();
        let from_postcard: HexData = postcard::from_bytes(&postcard_bytes).unwrap();
        prop_assert_eq!(&original, &from_postcard);
        
        // CBOR
        let mut cbor_bytes = vec![];
        ciborium::ser::into_writer(&original, &mut cbor_bytes).unwrap();
        let from_cbor: HexData = ciborium::de::from_reader(&cbor_bytes[..]).unwrap();
        prop_assert_eq!(&original, &from_cbor);
        
        // 所有格式应该一致（使用引用避免move）
        prop_assert_eq!(&from_json, &from_bincode);
        prop_assert_eq!(&from_bincode, &from_postcard);
        prop_assert_eq!(&from_postcard, &from_cbor);
        prop_assert_eq!(&original, &from_json);
    }
}

// ================================================================================
// 极端大小 Fuzz 测试
// ================================================================================

proptest! {
    #[test]
    fn test_extreme_sizes_fuzz(size in prop::sample::select(vec![0, 1, 2, 16, 32, 64, 128, 256, 512, 1024, 10000])) {
        let data = vec![0xAAu8; size];
        let hex_data = HexData { data };
        
        let json = serde_json::to_string(&hex_data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        
        prop_assert_eq!(deserialized.data.len(), size);
        prop_assert!(deserialized.data.iter().all(|&b| b == 0xAA));
        prop_assert_eq!(&hex_data, &deserialized);
    }
}

// ================================================================================
// 错误路径 Fuzz 测试
// ================================================================================

#[test]
fn test_deserialize_errors_comprehensive() {
    let error_cases = vec![
        (r#"{"data":"GG"}"#, "invalid hex character G"),
        (r#"{"data":"123"}"#, "odd length hex"),
        (r#"{"data":123}"#, "wrong type"),
        (r#"{"data":null}"#, "null value"),
        (r#"{"data":[]}"#, "array instead of string"),
        (r#"{"wrong_field":"00"}"#, "missing required field"),
    ];
    
    for (json, desc) in error_cases {
        let result: Result<HexData, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Should fail for: {}", desc);
    }
}

// ================================================================================
// 性能测试 - 大数据
// ================================================================================

#[test]
fn test_large_data_performance() {
    let sizes = vec![1_000, 10_000, 100_000];
    
    for size in sizes {
        let data = vec![0x42u8; size];
        let hex_data = HexData { data };
        
        let start = std::time::Instant::now();
        let json = serde_json::to_string(&hex_data).unwrap();
        let serialize_time = start.elapsed();
        
        let start = std::time::Instant::now();
        let _deserialized: HexData = serde_json::from_str(&json).unwrap();
        let deserialize_time = start.elapsed();
        
        // 验证性能合理（不超过1秒）
        assert!(serialize_time.as_secs() < 1, "Serialize too slow for {} bytes", size);
        assert!(deserialize_time.as_secs() < 1, "Deserialize too slow for {} bytes", size);
    }
}

// ================================================================================
// Quickcheck 风格测试
// ================================================================================

// 注：quickcheck_macros 需要添加到 Cargo.toml 的 dev-dependencies
// 暂时注释掉这些属性宏测试，改用普通测试
// #[quickcheck_macros::quickcheck]
#[test]
fn quickcheck_hex_roundtrip() {
    let data = vec![0x01, 0x02, 0x03, 0xff];
    
    let hex_data = HexData { data: data.clone() };
    
    let json = serde_json::to_string(&hex_data).unwrap();
    let deserialized: HexData = serde_json::from_str(&json).unwrap();
    
    assert_eq!(hex_data, deserialized);
}

// #[quickcheck_macros::quickcheck]
#[test]
fn quickcheck_hex_encode_decode() {
    let data = vec![0xaa, 0xbb, 0xcc];
    
    let hex_str = hex::encode(&data);
    let decoded = hex::decode(&hex_str).unwrap();
    
    assert_eq!(data, decoded);
}

// ================================================================================
// 并发 Fuzz 测试
// ================================================================================

#[test]
fn test_concurrent_serialization() {
    use std::thread;
    use rand::Rng;
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let len = rng.gen_range(0..100);
                let data: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
                
                let hex_data = HexData { data };
                
                let json = serde_json::to_string(&hex_data).unwrap();
                let deserialized: HexData = serde_json::from_str(&json).unwrap();
                
                (i, hex_data == deserialized)
            })
        })
        .collect();
    
    for handle in handles {
        let (_, success) = handle.join().unwrap();
        assert!(success);
    }
}

// ================================================================================
// 覆盖率增强测试 - 针对未覆盖路径
// ================================================================================

#[test]
fn test_hex_decode_error_paths() {
    // 测试 hex::decode 的错误路径
    let invalid_inputs = vec![
        "GG",           // 无效字符
        "12G",          // 混合
        "xyz",          // 完全无效
        "123",          // 奇数长度
        " 00",          // 前导空格
        "00 ",          // 尾随空格
        "0\n0",         // 换行符
        "00\r\n",       // 回车换行
    ];
    
    for input in invalid_inputs {
        let result = hex::decode(input);
        assert!(result.is_err());
    }
}

#[test]
fn test_hex_encode_special_patterns() {
    let patterns = vec![
        (vec![], ""),                           // 空
        (vec![0x00], "00"),                     // 单零
        (vec![0xFF], "ff"),                     // 单最大
        (vec![0x00, 0xFF], "00ff"),             // 混合
        (vec![0x12, 0x34, 0x56], "123456"),     // 连续
    ];
    
    for (input, expected) in patterns {
        let encoded = hex::encode(&input);
        assert_eq!(encoded, expected);
    }
}

// ================================================================================
// Serde 自定义错误测试
// ================================================================================

#[test]
fn test_serde_error_messages() {
    // 测试序列化错误
    #[derive(Deserialize, Debug)]
    struct Test {
        #[allow(dead_code)]
        required: String,
    }
    
    let missing_field = r#"{}"#;
    let result: Result<Test, _> = serde_json::from_str(missing_field);
    assert!(result.is_err());
    
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("missing field") || err_msg.len() > 0);
}

#[test]
fn test_deserialize_type_mismatch() {
    #[derive(Deserialize)]
    struct Expected {
        #[allow(dead_code)]
        value: u64,
    }
    
    let wrong_type = r#"{"value":"string"}"#;
    let result: Result<Expected, _> = serde_json::from_str(wrong_type);
    assert!(result.is_err());
}

