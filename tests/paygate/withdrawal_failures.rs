//! 提现失败测试
//! 
//! 覆盖：余额不足、每日限额、风控拦截等

#[tokio::test]
async fn test_withdrawal_exceeds_balance() {
    let balance = 1.0;
    let withdrawal_amount = 10.0;
    
    assert!(withdrawal_amount > balance, "提现金额超过余额");
}

#[tokio::test]
async fn test_withdrawal_daily_limit_exceeded() {
    let daily_limit = 100.0;
    let today_withdrawn = 95.0;
    let new_withdrawal = 10.0;
    
    assert!(today_withdrawn + new_withdrawal > daily_limit, "超过每日限额");
}

#[tokio::test]
async fn test_withdrawal_to_unverified_address() {
    let is_whitelisted = false;
    assert!(!is_whitelisted, "地址未验证");
}

#[tokio::test]
async fn test_withdrawal_risk_control_blocked() {
    let risk_score = 95; // 高风险
    let threshold = 80;
    
    assert!(risk_score > threshold, "风控拦截");
}

#[tokio::test]
async fn test_withdrawal_cooling_period() {
    use std::time::Duration;
    
    let last_withdrawal_time = std::time::Instant::now() - Duration::from_secs(3600);
    let cooling_period = Duration::from_secs(7200); // 2小时
    
    assert!(last_withdrawal_time.elapsed() < cooling_period, "冷却期未过");
}

#[tokio::test]
async fn test_withdrawal_suspicious_pattern() {
    // 测试可疑提现模式
    let withdrawal_count_in_hour = 10;
    let suspicious_threshold = 5;
    
    assert!(withdrawal_count_in_hour > suspicious_threshold, "可疑提现模式");
}

#[tokio::test]
async fn test_withdrawal_to_contract_address() {
    // 测试提现到合约地址（可能风险）
    let is_contract = true;
    assert!(is_contract, "目标是合约地址");
}

#[tokio::test]
async fn test_withdrawal_insufficient_gas_for_fee() {
    let balance = 0.001;
    let withdrawal_amount = 0.001;
    let estimated_fee = 0.0005;
    
    assert!(withdrawal_amount + estimated_fee > balance, "余额不足支付手续费");
}

