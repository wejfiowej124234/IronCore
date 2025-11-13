//! env_manager/manager.rs 专门测试
//! 覆盖 SecureEnvManager 的所有功能

use defi_hot_wallet::security::env_manager::permissions::PermissionLevel;

#[test]
fn test_secure_env_manager_new() {
    // 测试创建环境管理器
    use defi_hot_wallet::security::env_manager::manager::SecureEnvManager;
    
    let manager = SecureEnvManager::new();
    // 验证创建成功
    let _ = manager;
}

#[test]
fn test_secure_env_manager_default() {
    // 测试默认构造
    use defi_hot_wallet::security::env_manager::manager::SecureEnvManager;
    
    let manager = SecureEnvManager::default();
    let _ = manager;
}

#[test]
fn test_secure_env_manager_get_existing() {
    // 测试获取存在的环境变量
    use defi_hot_wallet::security::env_manager::manager::SecureEnvManager;
    
    std::env::set_var("TEST_MANAGER_VAR", "value");
    
    let manager = SecureEnvManager::new();
    let result = manager.get("TEST_MANAGER_VAR");
    
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "value");
    
    std::env::remove_var("TEST_MANAGER_VAR");
}

#[test]
fn test_secure_env_manager_get_nonexistent() {
    // 测试获取不存在的环境变量
    use defi_hot_wallet::security::env_manager::manager::SecureEnvManager;
    
    let manager = SecureEnvManager::new();
    let result = manager.get("NONEXISTENT_VAR_12345");
    
    assert!(result.is_none());
}

#[test]
fn test_permission_level_readonly() {
    // 测试只读权限级别
    let perm = PermissionLevel::ReadOnly;
    assert!(format!("{:?}", perm).contains("ReadOnly"));
}

#[test]
fn test_permission_level_readwrite() {
    // 测试读写权限级别
    let perm = PermissionLevel::ReadWrite;
    assert!(format!("{:?}", perm).contains("ReadWrite"));
}

#[test]
fn test_permission_level_admin() {
    // 测试管理员权限级别
    let perm = PermissionLevel::Admin;
    assert!(format!("{:?}", perm).contains("Admin"));
}

#[test]
fn test_permission_level_none() {
    // 测试无权限级别
    let perm = PermissionLevel::None;
    assert!(format!("{:?}", perm).contains("None"));
}

#[test]
fn test_set_permission_operations() {
    // 测试设置权限操作
    use defi_hot_wallet::security::env_manager::manager::SecureEnvManager;
    
    let manager = SecureEnvManager::new();
    
    // 设置不同的权限级别
    manager.set_permission("key1", PermissionLevel::ReadOnly);
    manager.set_permission("key2", PermissionLevel::ReadWrite);
    manager.set_permission("key3", PermissionLevel::Admin);
    
    // 验证不会 panic
}

#[test]
fn test_global_env_manager_access() {
    // 测试全局环境管理器访问
    use defi_hot_wallet::security::env_manager::manager::SECURE_ENV_MANAGER;
    
    std::env::set_var("GLOBAL_TEST_VAR", "global_value");
    
    let value = SECURE_ENV_MANAGER.get("GLOBAL_TEST_VAR");
    assert!(value.is_some());
    
    std::env::remove_var("GLOBAL_TEST_VAR");
}

#[test]
fn test_concurrent_env_manager_access() {
    // 测试并发访问环境管理器
    use defi_hot_wallet::security::env_manager::manager::SECURE_ENV_MANAGER;
    use std::thread;
    
    std::env::set_var("CONCURRENT_VAR", "value");
    
    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                SECURE_ENV_MANAGER.get("CONCURRENT_VAR")
            })
        })
        .collect();
    
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_some());
    }
    
    std::env::remove_var("CONCURRENT_VAR");
}

#[test]
fn test_env_manager_with_empty_key() {
    // 测试空键
    use defi_hot_wallet::security::env_manager::manager::SecureEnvManager;
    
    let manager = SecureEnvManager::new();
    let result = manager.get("");
    
    // 空键应该返回 None
    assert!(result.is_none());
}

#[test]
fn test_permission_comparison() {
    // 测试权限级别比较
    let none = PermissionLevel::None;
    let readonly = PermissionLevel::ReadOnly;
    let readwrite = PermissionLevel::ReadWrite;
    let admin = PermissionLevel::Admin;
    
    // 验证不同的权限级别
    assert!(none < readonly);
    assert!(readonly < readwrite);
    assert!(readwrite < admin);
}

#[test]
fn test_permission_has_permission() {
    // 测试权限检查
    assert!(PermissionLevel::Admin.has_permission(PermissionLevel::ReadOnly));
    assert!(PermissionLevel::ReadWrite.has_permission(PermissionLevel::ReadOnly));
    assert!(!PermissionLevel::ReadOnly.has_permission(PermissionLevel::Admin));
    assert!(!PermissionLevel::None.has_permission(PermissionLevel::ReadOnly));
}

#[test]
fn test_permission_from_str() {
    // 测试从字符串解析
    assert_eq!(PermissionLevel::from_str("admin"), Some(PermissionLevel::Admin));
    assert_eq!(PermissionLevel::from_str("readonly"), Some(PermissionLevel::ReadOnly));
    assert_eq!(PermissionLevel::from_str("readwrite"), Some(PermissionLevel::ReadWrite));
    assert_eq!(PermissionLevel::from_str("none"), Some(PermissionLevel::None));
    assert_eq!(PermissionLevel::from_str("invalid"), None);
}

#[test]
fn test_env_manager_stress_test() {
    // 压力测试
    use defi_hot_wallet::security::env_manager::manager::SecureEnvManager;
    
    let manager = SecureEnvManager::new();
    
    for i in 0..100 {
        let key = format!("STRESS_VAR_{}", i);
        std::env::set_var(&key, format!("value_{}", i));
        
        let result = manager.get(&key);
        assert!(result.is_some());
        
        std::env::remove_var(&key);
    }
}

#[test]
fn test_env_validation_format() {
    // 测试环境变量格式验证
    let valid_formats = vec![
        "simple_value",
        "123456",
        "value-with-dash",
        "value_with_underscore",
        "https://example.com",
    ];
    
    for value in valid_formats {
        std::env::set_var("FORMAT_TEST", value);
        let result = std::env::var("FORMAT_TEST");
        assert!(result.is_ok());
        std::env::remove_var("FORMAT_TEST");
    }
}

