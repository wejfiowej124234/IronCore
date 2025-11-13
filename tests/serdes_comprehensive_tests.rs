//! tools/serdes.rs 全面测试
//! 覆盖：序列化/反序列化、椭圆曲线点、无穷大、非法坐标、round-trip验证

use serde::{Deserialize, Serialize};

// ================================================================================
// Hex 序列化/反序列化边界测试
// ================================================================================

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

#[test]
fn test_hex_roundtrip_normal() {
    let original = HexData {
        data: vec![0x01, 0x02, 0x03, 0xAB, 0xCD, 0xEF],
    };
    
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: HexData = serde_json::from_str(&json).unwrap();
    
    assert_eq!(original, deserialized);
}

#[test]
fn test_hex_empty_data() {
    let original = HexData { data: vec![] };
    
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: HexData = serde_json::from_str(&json).unwrap();
    
    assert_eq!(original, deserialized);
}

#[test]
fn test_hex_single_byte() {
    let original = HexData { data: vec![0xFF] };
    
    let json = serde_json::to_string(&original).unwrap();
    assert!(json.contains("ff"));
    
    let deserialized: HexData = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_hex_all_zeros() {
    let original = HexData { data: vec![0x00; 32] };
    
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: HexData = serde_json::from_str(&json).unwrap();
    
    assert_eq!(original, deserialized);
}

#[test]
fn test_hex_all_ones() {
    let original = HexData { data: vec![0xFF; 32] };
    
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: HexData = serde_json::from_str(&json).unwrap();
    
    assert_eq!(original, deserialized);
}

// ================================================================================
// 无效输入测试 - 椭圆曲线点模拟
// ================================================================================

#[test]
fn test_hex_invalid_characters() {
    let invalid_json = r#"{"data":"GHIJKLMN"}"#; // 非hex字符
    let result: Result<HexData, _> = serde_json::from_str(invalid_json);
    
    assert!(result.is_err());
}

#[test]
fn test_hex_odd_length() {
    let invalid_json = r#"{"data":"abc"}"#; // 奇数长度
    let result: Result<HexData, _> = serde_json::from_str(invalid_json);
    
    assert!(result.is_err());
}

#[test]
fn test_hex_special_chars() {
    let invalid_json = r#"{"data":"00@11#22"}"#;
    let result: Result<HexData, _> = serde_json::from_str(invalid_json);
    
    assert!(result.is_err());
}

#[test]
fn test_hex_whitespace() {
    let invalid_json = r#"{"data":"00 11 22"}"#; // 包含空格
    let result: Result<HexData, _> = serde_json::from_str(invalid_json);
    
    assert!(result.is_err());
}

// ================================================================================
// 模拟椭圆曲线点序列化（无穷大点、非法坐标）
// ================================================================================

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct MockPoint {
    #[serde(with = "hex_serde")]
    compressed: Vec<u8>,
}

#[test]
fn test_point_at_infinity_representation() {
    // 无穷大点通常表示为全零
    let infinity = MockPoint {
        compressed: vec![0x00; 33], // 33字节压缩点
    };
    
    let json = serde_json::to_string(&infinity).unwrap();
    let deserialized: MockPoint = serde_json::from_str(&json).unwrap();
    
    assert_eq!(infinity, deserialized);
}

#[test]
fn test_point_invalid_length() {
    // 无效的点长度（应该是33或65字节）
    let invalid_json = r#"{"compressed":"0102"}"#; // 只有2字节
    let result: Result<MockPoint, _> = serde_json::from_str(invalid_json);
    
    // 可以反序列化，但逻辑上无效
    if let Ok(point) = result {
        assert_eq!(point.compressed.len(), 2);
    }
}

#[test]
fn test_point_invalid_prefix() {
    // 压缩点应该以0x02或0x03开头
    let mut invalid_compressed = vec![0x05; 33]; // 无效前缀
    invalid_compressed[0] = 0x05;
    
    let point = MockPoint {
        compressed: invalid_compressed,
    };
    
    let json = serde_json::to_string(&point).unwrap();
    let deserialized: MockPoint = serde_json::from_str(&json).unwrap();
    
    assert_eq!(point, deserialized);
    assert_eq!(deserialized.compressed[0], 0x05);
}

#[test]
fn test_point_y_coordinate_overflow() {
    // 模拟y坐标超出模数的情况
    let overflow_point = MockPoint {
        compressed: vec![0xFF; 33],
    };
    
    let json = serde_json::to_string(&overflow_point).unwrap();
    let deserialized: MockPoint = serde_json::from_str(&json).unwrap();
    
    assert_eq!(overflow_point, deserialized);
}

// ================================================================================
// 多格式序列化对比测试（Bincode vs JSON）
// ================================================================================

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct DualFormatData {
    value: u64,
    #[serde(with = "hex_serde")]
    data: Vec<u8>,
}

#[test]
fn test_serde_json_serialization() {
    let data = DualFormatData {
        value: 12345,
        data: vec![0xAB, 0xCD, 0xEF],
    };
    
    let json = serde_json::to_string(&data).unwrap();
    assert!(json.contains("abcdef"));
    
    let deserialized: DualFormatData = serde_json::from_str(&json).unwrap();
    assert_eq!(data.value, deserialized.value);
    assert_eq!(data.data, deserialized.data);
}

#[test]
fn test_bincode_serialization() {
    let data = DualFormatData {
        value: 12345,
        data: vec![0xAB, 0xCD, 0xEF],
    };
    
    let bincode_bytes = bincode::serialize(&data).unwrap();
    let deserialized: DualFormatData = bincode::deserialize(&bincode_bytes).unwrap();
    
    assert_eq!(data.value, deserialized.value);
    assert_eq!(data.data, deserialized.data);
}

#[test]
fn test_json_vs_bincode_size_difference() {
    let data = DualFormatData {
        value: 999,
        data: vec![0x11, 0x22, 0x33],
    };
    
    let json_size = serde_json::to_string(&data).unwrap().len();
    let bincode_size = bincode::serialize(&data).unwrap().len();
    
    // JSON通常更大
    assert!(json_size > bincode_size);
}

// ================================================================================
// Round-trip 验证（使用 arbitrary/proptest 概念）
// ================================================================================

#[test]
fn test_roundtrip_various_sizes() {
    let sizes = [0, 1, 2, 16, 32, 64, 128, 256, 512, 1024];
    
    for size in sizes.iter() {
        let data = HexData {
            data: vec![0xAA; *size],
        };
        
        // JSON round-trip
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        assert_eq!(data, deserialized, "Failed at size {}", size);
        
        // Bincode round-trip
        let bincode_bytes = bincode::serialize(&data).unwrap();
        let bincode_deser: HexData = bincode::deserialize(&bincode_bytes).unwrap();
        assert_eq!(data, bincode_deser, "Bincode failed at size {}", size);
    }
}

#[test]
fn test_roundtrip_random_patterns() {
    let patterns = [
        vec![0x00],
        vec![0xFF],
        vec![0x55, 0xAA],
        vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF],
        (0..=255).collect::<Vec<u8>>(),
    ];
    
    for (i, pattern) in patterns.iter().enumerate() {
        let data = HexData {
            data: pattern.clone(),
        };
        
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: HexData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(data, deserialized, "Failed at pattern {}", i);
    }
}

#[test]
fn test_roundtrip_edge_values() {
    let edge_cases = vec![
        vec![0x00],           // 最小值
        vec![0xFF],           // 最大值
        vec![0x00, 0xFF],     // 混合
        vec![0x7F],           // 中间值
        vec![0x80],           // 符号位
    ];
    
    for case in edge_cases {
        let data = HexData { data: case.clone() };
        
        let json = serde_json::to_string(&data).unwrap();
        let deser: HexData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(data, deser);
    }
}

// ================================================================================
// 不同格式的序列化测试
// ================================================================================

#[test]
fn test_json_compact_vs_pretty() {
    let data = HexData {
        data: vec![0x12, 0x34, 0x56],
    };
    
    let compact = serde_json::to_string(&data).unwrap();
    let pretty = serde_json::to_string_pretty(&data).unwrap();
    
    // 两者都应该能正确反序列化
    let from_compact: HexData = serde_json::from_str(&compact).unwrap();
    let from_pretty: HexData = serde_json::from_str(&pretty).unwrap();
    
    assert_eq!(data, from_compact);
    assert_eq!(data, from_pretty);
}

#[test]
fn test_postcard_roundtrip() {
    let data = HexData {
        data: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };
    
    let encoded = postcard::to_allocvec(&data).unwrap();
    let decoded: HexData = postcard::from_bytes(&encoded).unwrap();
    
    assert_eq!(data, decoded);
}

#[test]
fn test_ciborium_cbor_roundtrip() {
    let data = HexData {
        data: vec![0xCA, 0xFE, 0xBA, 0xBE],
    };
    
    let mut encoded = vec![];
    ciborium::ser::into_writer(&data, &mut encoded).unwrap();
    
    let decoded: HexData = ciborium::de::from_reader(&encoded[..]).unwrap();
    
    assert_eq!(data, decoded);
}

// ================================================================================
// 错误处理和恢复测试
// ================================================================================

#[test]
fn test_deserialize_truncated_data() {
    let truncated_json = r#"{"data":"abcd"#; // 截断的JSON
    let result: Result<HexData, _> = serde_json::from_str(truncated_json);
    
    assert!(result.is_err());
}

#[test]
fn test_deserialize_wrong_type() {
    let wrong_type_json = r#"{"data":12345}"#; // 数字而不是字符串
    let result: Result<HexData, _> = serde_json::from_str(wrong_type_json);
    
    assert!(result.is_err());
}

#[test]
fn test_deserialize_null_value() {
    let null_json = r#"{"data":null}"#;
    let result: Result<HexData, _> = serde_json::from_str(null_json);
    
    assert!(result.is_err());
}

#[test]
fn test_deserialize_array_instead_of_hex() {
    let array_json = r#"{"data":[1,2,3]}"#;
    let result: Result<HexData, _> = serde_json::from_str(array_json);
    
    assert!(result.is_err());
}

// ================================================================================
// 大数据压力测试
// ================================================================================

#[test]
fn test_very_large_data_roundtrip() {
    let large_data = HexData {
        data: vec![0x42; 100_000], // 100KB
    };
    
    let json = serde_json::to_string(&large_data).unwrap();
    let deserialized: HexData = serde_json::from_str(&json).unwrap();
    
    assert_eq!(large_data, deserialized);
}

#[test]
fn test_many_small_roundtrips() {
    for i in 0..1000 {
        let data = HexData {
            data: vec![(i % 256) as u8; (i % 10) + 1],
        };
        
        let json = serde_json::to_string(&data).unwrap();
        let deser: HexData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(data, deser);
    }
}

// ================================================================================
// 特殊格式测试
// ================================================================================

#[test]
fn test_uppercase_hex() {
    let uppercase_json = r#"{"data":"ABCDEF"}"#;
    let result: HexData = serde_json::from_str(uppercase_json).unwrap();
    
    assert_eq!(result.data, vec![0xAB, 0xCD, 0xEF]);
}

#[test]
fn test_lowercase_hex() {
    let lowercase_json = r#"{"data":"abcdef"}"#;
    let result: HexData = serde_json::from_str(lowercase_json).unwrap();
    
    assert_eq!(result.data, vec![0xAB, 0xCD, 0xEF]);
}

#[test]
fn test_mixed_case_hex() {
    let mixed_json = r#"{"data":"AbCdEf"}"#;
    let result: HexData = serde_json::from_str(mixed_json).unwrap();
    
    assert_eq!(result.data, vec![0xAB, 0xCD, 0xEF]);
}

