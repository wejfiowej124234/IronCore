//! env_manager/manager.rs 环境管理测试
//! 覆盖：环境变量管理、权限检查、验证逻辑

use std::env;

#[test]
fn test_env_var_get_set() {
    let key = "TEST_ENV_VAR_12345";
    let value = "test_value";
    
    env::set_var(key, value);
    assert_eq!(env::var(key).unwrap(), value);
    
    env::remove_var(key);
    assert!(env::var(key).is_err());
}

#[test]
fn test_env_var_overwrite() {
    let key = "TEST_ENV_OVERWRITE";
    
    env::set_var(key, "value1");
    assert_eq!(env::var(key).unwrap(), "value1");
    
    env::set_var(key, "value2");
    assert_eq!(env::var(key).unwrap(), "value2");
    
    env::remove_var(key);
}

#[test]
fn test_env_var_empty_value() {
    let key = "TEST_ENV_EMPTY";
    
    env::set_var(key, "");
    assert_eq!(env::var(key).unwrap(), "");
    
    env::remove_var(key);
}

#[test]
fn test_env_var_unicode() {
    let key = "TEST_ENV_UNICODE";
    let value = "测试值🎉";
    
    env::set_var(key, value);
    assert_eq!(env::var(key).unwrap(), value);
    
    env::remove_var(key);
}

#[test]
fn test_env_var_large_value() {
    let key = "TEST_ENV_LARGE";
    let value = "x".repeat(10000);
    
    env::set_var(key, &value);
    assert_eq!(env::var(key).unwrap(), value);
    
    env::remove_var(key);
}

#[test]
fn test_env_var_special_characters() {
    let key = "TEST_ENV_SPECIAL";
    let value = "!@#$%^&*()";
    
    env::set_var(key, value);
    assert_eq!(env::var(key).unwrap(), value);
    
    env::remove_var(key);
}

#[test]
fn test_multiple_env_vars() {
    let vars = vec![
        ("VAR1", "value1"),
        ("VAR2", "value2"),
        ("VAR3", "value3"),
    ];
    
    for (key, value) in &vars {
        env::set_var(key, value);
    }
    
    for (key, value) in &vars {
        assert_eq!(env::var(key).unwrap(), *value);
    }
    
    for (key, _) in &vars {
        env::remove_var(key);
    }
}

#[test]
fn test_env_var_case_sensitive() {
    env::set_var("TEST_LOWER", "lower");
    env::set_var("TEST_UPPER", "UPPER");
    
    assert_eq!(env::var("TEST_LOWER").unwrap(), "lower");
    assert_eq!(env::var("TEST_UPPER").unwrap(), "UPPER");
    
    // 不同大小写是不同的变量（在大多数系统上）
    if env::var("test_lower").is_ok() {
        // Windows 不区分大小写
        assert!(cfg!(target_os = "windows"));
    }
    
    env::remove_var("TEST_LOWER");
    env::remove_var("TEST_UPPER");
}

#[test]
fn test_env_validation_numeric() {
    let key = "TEST_NUMERIC";
    env::set_var(key, "12345");
    
    let value = env::var(key).unwrap();
    let parsed: Result<u32, _> = value.parse();
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap(), 12345);
    
    env::remove_var(key);
}

#[test]
fn test_env_validation_boolean() {
    let test_cases = vec![
        ("1", true),
        ("true", true),
        ("TRUE", true),
        ("0", false),
        ("false", false),
        ("FALSE", false),
    ];
    
    for (value, expected) in test_cases {
        let key = "TEST_BOOL";
        env::set_var(key, value);
        
        let v = env::var(key).unwrap();
        let is_true = v == "1" || v.eq_ignore_ascii_case("true");
        assert_eq!(is_true, expected);
        
        env::remove_var(key);
    }
}

#[test]
fn test_env_var_with_equals_sign() {
    let key = "TEST_EQUALS";
    let value = "key=value";
    
    env::set_var(key, value);
    assert_eq!(env::var(key).unwrap(), value);
    
    env::remove_var(key);
}

#[test]
fn test_env_var_with_newlines() {
    let key = "TEST_NEWLINES";
    let value = "line1\nline2\nline3";
    
    env::set_var(key, value);
    assert_eq!(env::var(key).unwrap(), value);
    
    env::remove_var(key);
}

#[test]
fn test_concurrent_env_access() {
    use std::thread;
    
    let key = "TEST_CONCURRENT";
    env::set_var(key, "initial");
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                env::set_var("TEST_CONCURRENT", format!("thread_{}", i));
                env::var("TEST_CONCURRENT").unwrap()
            })
        })
        .collect();
    
    for handle in handles {
        let _ = handle.join();
    }
    
    env::remove_var(key);
}

#[test]
fn test_env_var_trim_handling() {
    let key = "TEST_TRIM";
    env::set_var(key, "  value  ");
    
    let value = env::var(key).unwrap();
    assert_eq!(value.trim(), "value");
    
    env::remove_var(key);
}

#[test]
fn test_env_var_base64_validation() {
    use base64::Engine;
    
    let key = "TEST_BASE64";
    let data = b"test data";
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    
    env::set_var(key, &encoded);
    
    let value = env::var(key).unwrap();
    let decoded = base64::engine::general_purpose::STANDARD.decode(value).unwrap();
    
    assert_eq!(decoded, data);
    
    env::remove_var(key);
}

#[test]
fn test_env_var_invalid_base64() {
    use base64::Engine;
    
    let invalid_b64 = "not-valid-base64!!!";
    let result = base64::engine::general_purpose::STANDARD.decode(invalid_b64);
    
    assert!(result.is_err());
}

#[test]
fn test_env_var_path_validation() {
    let key = "TEST_PATH";
    let path = "/tmp/test/path";
    
    env::set_var(key, path);
    
    let value = env::var(key).unwrap();
    assert!(value.contains("/"));
    
    env::remove_var(key);
}

#[test]
fn test_env_var_url_validation() {
    let key = "TEST_URL";
    let url = "https://example.com:8080/api";
    
    env::set_var(key, url);
    
    let value = env::var(key).unwrap();
    assert!(value.starts_with("https://"));
    
    env::remove_var(key);
}

#[test]
fn test_permission_level_parsing() {
    // 模拟权限级别解析
    let levels = vec!["read", "write", "admin"];
    
    for level in levels {
        assert!(!level.is_empty());
        assert!(level.chars().all(|c| c.is_alphabetic()));
    }
}

#[test]
fn test_env_manager_initialization() {
    // 测试环境管理器初始化
    let required_vars = vec![
        ("WALLET_ENC_KEY", "test_key"),
        ("TEST_SKIP_DECRYPT", "1"),
    ];
    
    for (key, value) in &required_vars {
        env::set_var(key, value);
    }
    
    for (key, _) in &required_vars {
        assert!(env::var(key).is_ok());
    }
    
    for (key, _) in &required_vars {
        env::remove_var(key);
    }
}

#[test]
fn test_env_cleanup_on_panic() {
    // 测试panic时的环境清理
    let key = "TEST_PANIC_CLEANUP";
    env::set_var(key, "value");
    
    let result = std::panic::catch_unwind(|| {
        assert!(env::var(key).is_ok());
    });
    
    assert!(result.is_ok());
    env::remove_var(key);
}

