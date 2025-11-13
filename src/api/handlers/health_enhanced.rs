//! 增强的健康check处理器
//!
//! 提供详细的系统健康状态监控

use axum::{extract::State, response::Json};
use std::sync::Arc;
use serde::Serialize;
use crate::api::docs::{HealthResponse, HealthChecks, ComponentHealth, MemoryHealth, DiskHealth};

/// 增强的健康check
/// 
/// check所有关键组件的健康状态：
/// - 数据库连接
/// - RPC节点连接
/// - 系统资源
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "健康checksuccess", body = HealthResponse),
    )
)]
pub async fn enhanced_health_check() -> Json<HealthResponse> {
    // 1. check数据库（这里简化处理）
    let db_health = check_database().await;
    
    // 2. check以太坊RPC
    let eth_rpc_health = check_ethereum_rpc().await;
    
    // 3. check比特币RPC（如果启用）
    let btc_rpc_health = None;  // 可选
    
    // 4. check系统资源
    let memory = check_memory();
    let disk = check_disk();
    
    // 5. 综合判断整体状态
    let overall_status = determine_overall_status(&db_health, &eth_rpc_health, &memory, &disk);
    
    Json(HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        checks: HealthChecks {
            database: db_health,
            rpc_ethereum: eth_rpc_health,
            rpc_bitcoin: btc_rpc_health,
            memory,
            disk,
        },
    })
}

/// check数据库连接
async fn check_database() -> ComponentHealth {
    let start = std::time::Instant::now();
    
    // Note:当前使用内存存储，无需check数据库
    // SQLite持久化时需添加数据库连接check
    ComponentHealth {
        status: "up".to_string(),
        latency_ms: Some(start.elapsed().as_millis() as u64),
        error: None,
    }
}

/// check以太坊RPC节点
async fn check_ethereum_rpc() -> ComponentHealth {
    use ethers::prelude::{Provider, Http, Middleware};
    
    let start = std::time::Instant::now();
    let rpc_url = "https://eth.llamarpc.com";  // 使用公共RPC
    
    match Provider::<Http>::try_from(rpc_url) {
        Ok(provider) => {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                provider.get_block_number()
            ).await {
                Ok(Ok(_block_number)) => {
                    ComponentHealth {
                        status: "up".to_string(),
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        error: None,
                    }
                }
                Ok(Err(e)) => {
                    ComponentHealth {
                        status: "down".to_string(),
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        error: Some(format!("RPCerror: {}", e)),
                    }
                }
                Err(_) => {
                    ComponentHealth {
                        status: "down".to_string(),
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        error: Some("RPC超时".to_string()),
                    }
                }
            }
        }
        Err(e) => {
            ComponentHealth {
                status: "down".to_string(),
                latency_ms: None,
                error: Some(format!("RPC连接failed: {}", e)),
            }
        }
    }
}

/// check内存使用情况
fn check_memory() -> MemoryHealth {
    use sysinfo::{System, SystemExt};
    
    let mut sys = System::new_all();
    sys.refresh_memory();
    
    let used = sys.used_memory() / 1024 / 1024;  // MB
    let total = sys.total_memory() / 1024 / 1024;  // MB
    let percentage = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    
    MemoryHealth {
        used_mb: used,
        total_mb: total,
        percentage,
    }
}

/// check磁盘使用情况
fn check_disk() -> DiskHealth {
    use sysinfo::{System, SystemExt, DiskExt};
    
    let mut sys = System::new_all();
    sys.refresh_disks_list();
    sys.refresh_disks();
    
    // fetch主磁盘信息
    let disks = sys.disks();
    if let Some(disk) = disks.first() {
        let available = disk.available_space() / 1024 / 1024 / 1024;  // GB
        let total = disk.total_space() / 1024 / 1024 / 1024;  // GB
        let percentage = if total > 0 {
            ((total - available) as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        
        DiskHealth {
            available_gb: available,
            total_gb: total,
            percentage,
        }
    } else {
        // 无法fetch磁盘信息时的默认值
        DiskHealth {
            available_gb: 0,
            total_gb: 0,
            percentage: 0.0,
        }
    }
}

/// 综合判断整体健康状态
fn determine_overall_status(
    db: &ComponentHealth,
    eth_rpc: &ComponentHealth,
    memory: &MemoryHealth,
    disk: &DiskHealth,
) -> String {
    // 关键组件down -> unhealthy
    if db.status == "down" {
        return "unhealthy".to_string();
    }
    
    // RPC down -> degraded
    if eth_rpc.status == "down" {
        return "degraded".to_string();
    }
    
    // 内存使用超过90% -> degraded
    if memory.percentage > 90.0 {
        return "degraded".to_string();
    }
    
    // 磁盘使用超过95% -> degraded
    if disk.percentage > 95.0 {
        return "degraded".to_string();
    }
    
    "healthy".to_string()
}

