// Shamir秘密分享 - 测试分片丢失恢复阈值

use defi_hot_wallet::security::shamir::{split_secret, combine_secret, ShamirError};

// === 1. 阈值恢复测试 ===

#[test]
fn test_shamir_threshold_2_of_3_success() {
    let secret = b"my_32_byte_secret_key_here!!!!!!";
    
    // 分割成3份，阈值2
    let shares = split_secret(secret, 2, 3).unwrap();
    assert_eq!(shares.len(), 3);
    
    // 使用任意2份恢复
    let selected = vec![shares[0].clone(), shares[1].clone()];
    let reconstructed = combine_secret(&selected, 2).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice(), "2/3阈值应该能恢复");
}

#[test]
fn test_shamir_threshold_2_of_3_another_combination() {
    let secret = b"another_32byte_secret_key_test!!";
    
    let shares = split_secret(secret, 2, 3).unwrap();
    
    // 使用不同的2份
    let selected = vec![shares[0].clone(), shares[2].clone()];
    let reconstructed = combine_secret(&selected, 2).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice(), "不同的2份也应该能恢复");
}

#[test]
fn test_shamir_threshold_3_of_5_success() {
    let secret = b"test_secret_32_bytes_long!!!!!!!" ;
    
    // 分割成5份，阈值3
    let shares = split_secret(secret, 3, 5).unwrap();
    assert_eq!(shares.len(), 5);
    
    // 使用3份恢复
    let selected = vec![shares[0].clone(), shares[2].clone(), shares[4].clone()];
    let reconstructed = combine_secret(&selected, 3).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice(), "3/5阈值应该能恢复");
}

// === 2. 分片丢失场景测试 ===

#[test]
fn test_shamir_insufficient_shares_1_of_2() {
    let secret = b"secret_key_32_bytes_long!!!!!!!!";
    
    let shares = split_secret(secret, 2, 3).unwrap();
    
    // 只有1份，阈值是2
    let selected = vec![shares[0].clone()];
    let result = combine_secret(&selected, 2);
    
    // 分片不足应该失败
    assert!(result.is_err(), "1/2分片不足应该失败");
}

#[test]
fn test_shamir_insufficient_shares_2_of_3() {
    let secret = b"test_32_byte_secret_key_here!!!!";
    
    let shares = split_secret(secret, 3, 5).unwrap();
    
    // 只有2份，阈值是3
    let selected = vec![shares[0].clone(), shares[1].clone()];
    let result = combine_secret(&selected, 3);
    
    // 分片不足应该失败
    assert!(result.is_err(), "2/3分片不足应该失败");
}

#[test]
fn test_shamir_all_shares_lost() {
    // 所有分片都丢失
    let empty_shares: Vec<(u8, [u8; 32])> = vec![];
    let result = combine_secret(&empty_shares, 2);
    
    assert!(result.is_err(), "没有分片应该失败");
}

// === 3. 阈值边界测试 ===

#[test]
fn test_shamir_threshold_equals_total_shares() {
    let secret = b"boundary_test_32bytes_secret!!!!";
    
    // 阈值 = 总数（需要所有分片）
    let shares = split_secret(secret, 3, 3).unwrap();
    
    let reconstructed = combine_secret(&shares, 3).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice(), "k=n应该能恢复");
}

#[test]
fn test_shamir_threshold_1_of_n() {
    let secret = b"min_threshold_32byte_secret!!!!!";
    
    // 最小阈值1
    let shares = split_secret(secret, 1, 5).unwrap();
    
    // 只用1份恢复
    let selected = vec![shares[0].clone()];
    let reconstructed = combine_secret(&selected, 1).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice(), "1/n阈值应该能恢复");
}

#[test]
fn test_shamir_threshold_zero_error() {
    let secret = b"zero_threshold_32byte_key!!!!!!!";
    
    // 阈值为0应该失败
    let result = split_secret(secret, 0, 3);
    
    assert!(result.is_err(), "阈值0应该失败");
    assert!(matches!(result.unwrap_err(), ShamirError::InvalidParameters(_)));
}

#[test]
fn test_shamir_total_shares_zero_error() {
    let secret = b"zero_shares_32byte_key!!!!!!!!!!";
    
    // 总份数为0应该失败
    let result = split_secret(secret, 2, 0);
    
    assert!(result.is_err(), "总份数0应该失败");
    assert!(matches!(result.unwrap_err(), ShamirError::InvalidParameters(_)));
}

#[test]
fn test_shamir_threshold_greater_than_total() {
    let secret = b"invalid_threshold_32byte_key!!!!";
    
    // 阈值 > 总数
    let result = split_secret(secret, 5, 3);
    
    assert!(result.is_err(), "k > n应该失败");
    assert!(matches!(result.unwrap_err(), ShamirError::InvalidParameters(_)));
}

// === 4. 秘密长度验证 ===

#[test]
fn test_shamir_secret_not_32_bytes() {
    let short_secret = b"short";
    
    let result = split_secret(short_secret, 2, 3);
    
    assert!(result.is_err(), "非32字节秘密应该失败");
    assert!(matches!(result.unwrap_err(), ShamirError::InvalidParameters(_)));
}

#[test]
fn test_shamir_secret_too_long() {
    let long_secret = vec![1u8; 64]; // 64字节
    
    let result = split_secret(&long_secret, 2, 3);
    
    assert!(result.is_err(), "超过32字节应该失败");
}

#[test]
fn test_shamir_secret_empty() {
    let empty: &[u8] = b"";
    
    let result = split_secret(empty, 2, 3);
    
    assert!(result.is_err(), "空秘密应该失败");
}

// === 5. 恢复场景测试 ===

#[test]
fn test_shamir_recover_with_exact_threshold() {
    let secret = b"exact_threshold_test_32bytes!!!!";
    
    let shares = split_secret(secret, 3, 5).unwrap();
    
    // 恰好3份（阈值）
    let selected = vec![shares[1].clone(), shares[2].clone(), shares[3].clone()];
    let reconstructed = combine_secret(&selected, 3).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice());
}

#[test]
fn test_shamir_recover_with_more_than_threshold() {
    let secret = b"more_than_threshold_32bytes!!!!!";
    
    let shares = split_secret(secret, 2, 5).unwrap();
    
    // 使用4份（超过阈值2）
    let selected = vec![
        shares[0].clone(),
        shares[1].clone(),
        shares[2].clone(),
        shares[4].clone(),
    ];
    let reconstructed = combine_secret(&selected, 2).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice(), "超过阈值的分片也应该能恢复");
}

#[test]
fn test_shamir_recover_all_shares() {
    let secret = b"all_shares_test_32byte_key!!!!!!";
    
    let shares = split_secret(secret, 2, 4).unwrap();
    
    // 使用全部4份
    let reconstructed = combine_secret(&shares, 2).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice(), "使用全部分片应该能恢复");
}

// === 6. 分片丢失组合测试 ===

#[test]
fn test_shamir_lose_first_share() {
    let secret = b"lose_first_32byte_secret!!!!!!!!";
    
    let shares = split_secret(secret, 2, 4).unwrap();
    
    // 丢失第1份，使用第2、3份
    let selected = vec![shares[1].clone(), shares[2].clone()];
    let reconstructed = combine_secret(&selected, 2).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice());
}

#[test]
fn test_shamir_lose_middle_shares() {
    let secret = b"lose_middle_32byte_secret!!!!!!!";
    
    let shares = split_secret(secret, 2, 5).unwrap();
    
    // 丢失中间的，使用第1和第5份
    let selected = vec![shares[0].clone(), shares[4].clone()];
    let reconstructed = combine_secret(&selected, 2).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice());
}

#[test]
fn test_shamir_lose_last_share() {
    let secret = b"lose_last_32byte_secret!!!!!!!!!";
    
    let shares = split_secret(secret, 2, 4).unwrap();
    
    // 丢失最后一份
    let selected = vec![shares[0].clone(), shares[1].clone()];
    let reconstructed = combine_secret(&selected, 2).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice());
}

// === 7. 极限阈值测试 ===

#[test]
fn test_shamir_max_shares_255() {
    let secret = b"max_shares_32byte_secret!!!!!!!!";
    
    // GF(2^8)最多255份
    let shares = split_secret(secret, 128, 255).unwrap();
    assert_eq!(shares.len(), 255);
    
    // 使用128份恢复
    let selected: Vec<_> = shares.iter().cloned().take(128).collect();
    let reconstructed = combine_secret(&selected, 128).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice());
}

#[test]
fn test_shamir_min_threshold_1_max_shares() {
    let secret = b"min_k_max_n_32byte_secret!!!!!!!";
    
    let shares = split_secret(secret, 1, 100).unwrap();
    
    // 只用1份就能恢复
    let selected = vec![shares[50].clone()];
    let reconstructed = combine_secret(&selected, 1).unwrap();
    
    assert_eq!(secret, reconstructed.as_slice());
}

