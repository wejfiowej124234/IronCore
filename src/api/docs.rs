//! OpenAPI 文档配置
//!
//! 使用 utoipa 自动生成 Swagger UI 文档

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "DeFi Hot Wallet API",
        version = env!("CARGO_PKG_VERSION"),
        description = "企业级加密货币wallet API\n\n## 功能特性\n\n- ✅ BIP39 mnemonic生成\n- ✅ 以太坊/比特币address派生\n- ✅ 真实的区块链transactionsign\n- ✅ AES-256-GCM 密钥加密\n- ✅ bcrypt Password哈希\n- ✅ JWT 认证\n- ✅ Prometheus 监控\n\n## 安全性\n\n- 🔒 mnemonic不存储在服务器\n- 🔒 Private key加密存储\n- 🔒 transaction前Passwordvalidate\n- 🔒 敏感内存自动清零\n\n## 认证\n\n大多数 API 需要在 Header 中包含认证令牌：\n```\nAuthorization: Bearer <your_token>\n```",
        contact(
            name = "API Support",
            email = "support@example.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "本地开发环境"),
        (url = "https://api.example.com", description = "生产环境")
    ),
    tags(
        (name = "health", description = "健康check和监控"),
        (name = "wallets", description = "wallet管理 - 创建、query、Delete Wallet"),
        (name = "transactions", description = "transaction操作 - 发送transaction、query历史"),
        (name = "auth", description = "user认证 - 注册、登录、令牌管理"),
        (name = "backup", description = "备份恢复 - wallet备份和恢复")
    ),
    components(
        schemas(
            // Types
            crate::api::types::CreateWalletRequest,
            crate::api::types::WalletResponse,
            crate::api::types::SendTransactionRequest,
            crate::api::types::TransactionResponse,
            crate::api::types::BalanceResponse,
            crate::api::types::TransactionHistoryResponse,
            crate::api::types::ErrorResponse,
            crate::api::types::RestoreWalletRequest,
            crate::api::types::MultiSigTransactionRequest,
            crate::api::types::MultiSigTransactionResponse,
            // Health
            HealthResponse,
            ComponentHealth,
            MemoryHealth,
            DiskHealth,
        ),
        responses(
            (status = 200, description = "请求success"),
            (status = 400, description = "请求参数error", body = crate::api::types::ErrorResponse),
            (status = 401, description = "未授权，需要认证", body = crate::api::types::ErrorResponse),
            (status = 404, description = "资源不存在", body = crate::api::types::ErrorResponse),
            (status = 500, description = "服务器内部error", body = crate::api::types::ErrorResponse),
        ),
        security_schemes(
            ("bearer_auth" = (type = http, scheme = bearer, bearer_format = "JWT"))
        )
    )
)]
pub struct ApiDoc;

/// 健康check响应
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// 整体状态: "healthy" | "degraded" | "unhealthy"
    #[schema(example = "healthy")]
    pub status: String,
    
    /// 服务版本
    #[schema(example = "0.1.0")]
    pub version: String,
    
    /// 时间戳
    #[schema(example = "2025-10-29T10:00:00Z")]
    pub timestamp: String,
    
    /// 各组件健康状态
    pub checks: HealthChecks,
}

/// 各组件健康check
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthChecks {
    /// 数据库状态
    pub database: ComponentHealth,
    
    /// 以太坊RPC状态
    pub rpc_ethereum: ComponentHealth,
    
    /// 比特币RPC状态（可选）
    pub rpc_bitcoin: Option<ComponentHealth>,
    
    /// 内存使用情况
    pub memory: MemoryHealth,
    
    /// 磁盘使用情况
    pub disk: DiskHealth,
}

/// 组件健康状态
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ComponentHealth {
    /// 状态: "up" | "down" | "degraded"
    #[schema(example = "up")]
    pub status: String,
    
    /// 响应延迟（毫秒）
    #[schema(example = 15)]
    pub latency_ms: Option<u64>,
    
    /// error信息
    pub error: Option<String>,
}

/// 内存健康状态
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct MemoryHealth {
    /// 已使用内存（MB）
    #[schema(example = 512)]
    pub used_mb: u64,
    
    /// 总内存（MB）
    #[schema(example = 8192)]
    pub total_mb: u64,
    
    /// 使用百分比
    #[schema(example = 6.25)]
    pub percentage: f32,
}

/// 磁盘健康状态
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DiskHealth {
    /// 可用空间（GB）
    #[schema(example = 100)]
    pub available_gb: u64,
    
    /// 总空间（GB）
    #[schema(example = 500)]
    pub total_gb: u64,
    
    /// 使用百分比
    #[schema(example = 80.0)]
    pub percentage: f32,
}

