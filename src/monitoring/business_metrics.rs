//! 业务指标记录器
//!
//! 提供业务级别的监控指标

use prometheus::{Counter, Histogram, Registry, Encoder, TextEncoder};
use std::sync::Arc;
use parking_lot::Mutex;

/// 业务指标管理器
pub struct BusinessMetrics {
    registry: Arc<Mutex<Registry>>,
    
    // wallet操作
    wallets_created_total: Counter,
    wallets_accessed_total: Counter,
    wallets_deleted_total: Counter,
    
    // transaction操作
    transactions_sent_total: Counter,
    transactions_failed_total: Counter,
    transaction_value_eth: Histogram,
    transaction_fees_eth: Histogram,
    transaction_duration_ms: Histogram,
    
    // 认证操作
    login_attempts_total: Counter,
    failed_logins_total: Counter,
    token_generated_total: Counter,
    
    // 区块链调用
    blockchain_rpc_calls_total: Counter,
    blockchain_rpc_errors_total: Counter,
    blockchain_rpc_latency_ms: Histogram,
}

impl BusinessMetrics {
    /// 创建新的业务指标管理器
    pub fn new() -> anyhow::Result<Self> {
        let registry = Arc::new(Mutex::new(Registry::new()));
        
        // 注册wallet指标
        let wallets_created_total = Counter::new(
            "wallets_created_total",
            "wallet创建总数"
        )?;
        let wallets_accessed_total = Counter::new(
            "wallets_accessed_total",
            "wallet访问总数"
        )?;
        let wallets_deleted_total = Counter::new(
            "wallets_deleted_total",
            "walletDelete总数"
        )?;
        
        // 注册transaction指标
        let transactions_sent_total = Counter::new(
            "transactions_sent_total",
            "transaction发送总数"
        )?;
        let transactions_failed_total = Counter::new(
            "transactions_failed_total",
            "transactionfailed总数"
        )?;
        let transaction_value_eth = Histogram::with_opts(
            prometheus::HistogramOpts::new("transaction_value_eth", "transaction金额(ETH)")
                .buckets(vec![0.01, 0.1, 1.0, 10.0, 100.0])
        )?;
        let transaction_fees_eth = Histogram::with_opts(
            prometheus::HistogramOpts::new("transaction_fees_eth", "transaction手续费(ETH)")
                .buckets(vec![0.001, 0.01, 0.1, 1.0])
        )?;
        let transaction_duration_ms = Histogram::with_opts(
            prometheus::HistogramOpts::new("transaction_duration_ms", "transaction处理时长(ms)")
                .buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0])
        )?;
        
        // 注册认证指标
        let login_attempts_total = Counter::new(
            "login_attempts_total",
            "登录尝试总数"
        )?;
        let failed_logins_total = Counter::new(
            "failed_logins_total",
            "登录failed总数"
        )?;
        let token_generated_total = Counter::new(
            "token_generated_total",
            "令牌生成总数"
        )?;
        
        // 注册区块链调用指标
        let blockchain_rpc_calls_total = Counter::new(
            "blockchain_rpc_calls_total",
            "区块链RPC调用总数"
        )?;
        let blockchain_rpc_errors_total = Counter::new(
            "blockchain_rpc_errors_total",
            "区块链RPCerror总数"
        )?;
        let blockchain_rpc_latency_ms = Histogram::with_opts(
            prometheus::HistogramOpts::new("blockchain_rpc_latency_ms", "RPC调用延迟(ms)")
                .buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0])
        )?;
        
        // 注册所有指标到registry
        {
            let r = registry.lock();
            r.register(Box::new(wallets_created_total.clone()))?;
            r.register(Box::new(wallets_accessed_total.clone()))?;
            r.register(Box::new(wallets_deleted_total.clone()))?;
            r.register(Box::new(transactions_sent_total.clone()))?;
            r.register(Box::new(transactions_failed_total.clone()))?;
            r.register(Box::new(transaction_value_eth.clone()))?;
            r.register(Box::new(transaction_fees_eth.clone()))?;
            r.register(Box::new(transaction_duration_ms.clone()))?;
            r.register(Box::new(login_attempts_total.clone()))?;
            r.register(Box::new(failed_logins_total.clone()))?;
            r.register(Box::new(token_generated_total.clone()))?;
            r.register(Box::new(blockchain_rpc_calls_total.clone()))?;
            r.register(Box::new(blockchain_rpc_errors_total.clone()))?;
            r.register(Box::new(blockchain_rpc_latency_ms.clone()))?;
        }
        
        Ok(Self {
            registry,
            wallets_created_total,
            wallets_accessed_total,
            wallets_deleted_total,
            transactions_sent_total,
            transactions_failed_total,
            transaction_value_eth,
            transaction_fees_eth,
            transaction_duration_ms,
            login_attempts_total,
            failed_logins_total,
            token_generated_total,
            blockchain_rpc_calls_total,
            blockchain_rpc_errors_total,
            blockchain_rpc_latency_ms,
        })
    }
    
    /// 记录wallet创建
    pub fn record_wallet_created(&self) {
        self.wallets_created_total.inc();
    }
    
    /// 记录wallet访问
    pub fn record_wallet_accessed(&self) {
        self.wallets_accessed_total.inc();
    }
    
    /// 记录walletDelete
    pub fn record_wallet_deleted(&self) {
        self.wallets_deleted_total.inc();
    }
    
    /// 记录transaction
    pub fn record_transaction(
        &self,
        value_eth: f64,
        fee_eth: f64,
        duration_ms: u64,
        success: bool,
    ) {
        if success {
            self.transactions_sent_total.inc();
            self.transaction_value_eth.observe(value_eth);
            self.transaction_fees_eth.observe(fee_eth);
        } else {
            self.transactions_failed_total.inc();
        }
        self.transaction_duration_ms.observe(duration_ms as f64);
    }
    
    /// 记录登录尝试
    pub fn record_login_attempt(&self, success: bool) {
        self.login_attempts_total.inc();
        if !success {
            self.failed_logins_total.inc();
        }
    }
    
    /// 记录令牌生成
    pub fn record_token_generated(&self) {
        self.token_generated_total.inc();
    }
    
    /// 记录区块链RPC调用
    pub fn record_blockchain_rpc_call(&self, latency_ms: u64, success: bool) {
        self.blockchain_rpc_calls_total.inc();
        if !success {
            self.blockchain_rpc_errors_total.inc();
        }
        self.blockchain_rpc_latency_ms.observe(latency_ms as f64);
    }
    
    /// 导出指标为Prometheus格式
    pub fn export(&self) -> anyhow::Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.lock().gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

impl Default for BusinessMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create BusinessMetrics")
    }
}

