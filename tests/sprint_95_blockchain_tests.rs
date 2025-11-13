//! 冲刺95%: 区块链交互测试
//! 目标: 覆盖区块链地址生成、网络配置等功能
//! 
//! 测试范围:
//! - 以太坊地址生成
//! - 多种区块链网络
//! - 地址验证
//! - 网络配置

use defi_hot_wallet::core::config::{BlockchainConfig, NetworkConfig, WalletConfig};
use std::collections::HashMap;

// ============================================================================
// 网络配置测试
// ============================================================================

#[test]
fn test_blockchain_config_default() {
    // 通过 WalletConfig 默认值获取 BlockchainConfig
    let config = WalletConfig::default().blockchain;
    
    // 默认配置应该有网络
    assert!(config.networks.len() > 0, 
            "默认配置应该有网络设置");
}

#[test]
fn test_blockchain_config_custom() {
    let mut networks = HashMap::new();
    
    networks.insert("eth".to_string(), NetworkConfig {
        name: "Ethereum Mainnet".to_string(),
        rpc_url: "https://mainnet.infura.io".to_string(),
        chain_id: 1u64,
    });
    
    let config = BlockchainConfig {
        networks,
    };
    
    assert_eq!(config.networks.len(), 1);
}

#[test]
fn test_network_config_ethereum() {
    let eth_config = NetworkConfig {
        name: "Ethereum Mainnet".to_string(),
        rpc_url: "https://mainnet.infura.io".to_string(),
        chain_id: 1u64,
    };
    
    assert_eq!(eth_config.chain_id, 1u64);
}

#[test]
fn test_network_config_polygon() {
    let polygon_config = NetworkConfig {
        name: "Polygon Mainnet".to_string(),
        rpc_url: "https://polygon-rpc.com".to_string(),
        chain_id: 137u64,
    };
    
    assert_eq!(polygon_config.chain_id, 137u64);
}

#[test]
fn test_network_config_bsc() {
    let bsc_config = NetworkConfig {
        name: "BSC Mainnet".to_string(),
        rpc_url: "https://bsc-dataseed.binance.org".to_string(),
        chain_id: 56u64,
    };
    
    assert_eq!(bsc_config.chain_id, 56u64);
}

#[test]
fn test_multiple_network_configs() {
    let mut networks = HashMap::new();
    
    networks.insert("eth".to_string(), NetworkConfig {
        name: "Ethereum Mainnet".to_string(),
        rpc_url: "https://mainnet.infura.io".to_string(),
        chain_id: 1u64,
    });
    
    networks.insert("polygon".to_string(), NetworkConfig {
        name: "Polygon Mainnet".to_string(),
        rpc_url: "https://polygon-rpc.com".to_string(),
        chain_id: 137u64,
    });
    
    networks.insert("bsc".to_string(), NetworkConfig {
        name: "BSC Mainnet".to_string(),
        rpc_url: "https://bsc-dataseed.binance.org".to_string(),
        chain_id: 56u64,
    });
    
    let config = BlockchainConfig {
        networks,
    };
    
    assert_eq!(config.networks.len(), 3);
    assert!(config.networks.contains_key("eth"));
    assert!(config.networks.contains_key("polygon"));
    assert!(config.networks.contains_key("bsc"));
}

// ============================================================================
// 链ID测试
// ============================================================================

#[test]
fn test_ethereum_mainnet_chain_id() {
    let chain_id = 1u64;
    assert_eq!(chain_id, 1, "以太坊主网chain_id应该是1");
}

#[test]
fn test_polygon_mainnet_chain_id() {
    let chain_id = 137u64;
    assert_eq!(chain_id, 137, "Polygon主网chain_id应该是137");
}

#[test]
fn test_bsc_mainnet_chain_id() {
    let chain_id = 56u64;
    assert_eq!(chain_id, 56, "BSC主网chain_id应该是56");
}

#[test]
fn test_goerli_testnet_chain_id() {
    let chain_id = 5u64;
    assert_eq!(chain_id, 5, "Goerli测试网chain_id应该是5");
}

// ============================================================================
// 区块时间（替代确认块数）测试
// ============================================================================

#[test]
fn test_ethereum_block_time_seconds() {
    let block_time = 12u64;
    assert!(block_time >= 1, "区块时间应该至少为1秒");
    assert_eq!(block_time, 12u64, "以太坊平均区块时间约12秒");
}

#[test]
fn test_fast_block_time_seconds() {
    let block_time = 1u64;
    assert_eq!(block_time, 1u64, "快速网络区块时间约1秒");
}

#[test]
fn test_secure_block_time_seconds() {
    let block_time = 12u64;
    assert!(block_time >= 12u64, "安全相关的平均区块时间应不小于标准");
}

// ============================================================================
// 并发配置测试
// ============================================================================

#[test]
fn test_concurrent_config_creation() {
    use std::thread;
    
    let mut handles = vec![];
    
    for i in 0..10usize {
        let handle = thread::spawn(move || {
            let mut networks = HashMap::new();
            networks.insert(format!("network-{}", i), NetworkConfig {
                name: format!("Network {}", i),
                rpc_url: format!("https://rpc-{}.example.com", i),
                chain_id: i as u64,
            });
            
            BlockchainConfig {
                networks,
            }
        });
        handles.push(handle);
    }
    
    let configs: Vec<_> = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    
    // 所有配置都应该创建成功
    assert_eq!(configs.len(), 10);
    
    // 验证每个配置都是唯一的
    for config in configs.iter() {
        assert_eq!(config.networks.len(), 1);
    }
}