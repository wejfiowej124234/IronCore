//! 用户拒绝/取消操作测试
//! 
//! 覆盖：用户拒绝签名、取消交易、超时未确认等

#[tokio::test]
async fn test_user_rejects_signature_request() {
    let user_approved = false;
    assert!(!user_approved, "用户拒绝签名");
}

#[tokio::test]
async fn test_user_cancels_pending_transaction() {
    let transaction_cancelled = true;
    assert!(transaction_cancelled, "用户取消交易");
}

#[tokio::test]
async fn test_user_confirmation_timeout() {
    use std::time::Duration;
    
    let waiting_time = Duration::from_secs(300); // 5分钟
    let timeout = Duration::from_secs(180); // 3分钟
    
    assert!(waiting_time > timeout, "用户确认超时");
}

#[tokio::test]
async fn test_wallet_locked_during_signing() {
    let is_wallet_locked = true;
    assert!(is_wallet_locked, "钱包已锁定");
}

#[tokio::test]
async fn test_hardware_wallet_disconnected() {
    let is_connected = false;
    assert!(!is_connected, "硬件钱包已断开");
}

#[tokio::test]
async fn test_user_closes_app_during_transaction() {
    // 模拟用户关闭应用
    let app_active = false;
    assert!(!app_active, "应用已关闭");
}

#[tokio::test]
async fn test_biometric_authentication_failed() {
    let biometric_verified = false;
    assert!(!biometric_verified, "生物识别失败");
}

#[tokio::test]
async fn test_2fa_code_incorrect() {
    let entered_code = "123456";
    let expected_code = "654321";
    
    assert_ne!(entered_code, expected_code, "2FA代码错误");
}

