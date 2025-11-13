//! AI异常检测 API 路由
//!
//! 提供REST和WebSocket接口供前端调用

use axum::{
    extract::{Json, State, WebSocketUpgrade, ws::{WebSocket, Message}},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures::{stream::StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::anomaly_detection::{
    AnomalyDetector, ThreatLevel,
};

/// API 状态
#[derive(Clone)]
pub struct AnomalyApiState {
    /// 异常检测器
    pub detector: Arc<tokio::sync::Mutex<AnomalyDetector>>,
    /// 事件广播通道
    pub event_tx: broadcast::Sender<AnomalyEventMessage>,
}

impl Default for AnomalyApiState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnomalyApiState {
    /// 创建新的异常检测 API 状态
    pub fn new() -> Self {
        let detector = Arc::new(tokio::sync::Mutex::new(AnomalyDetector::new()));
        let (event_tx, _) = broadcast::channel(100);
        Self {
            detector,
            event_tx,
        }
    }
}

/// 检测请求
#[derive(Debug, Deserialize)]
pub struct DetectionRequest {
    /// 目标address
    pub to_address: String,
    /// 转账金额
    pub amount: f64,
    /// Gas price（可选）
    pub gas_price: Option<u64>,
    /// 是否为合约调用
    #[serde(default)]
    pub is_contract: bool,
    /// 来源address（可选）
    pub from_address: Option<String>,
    /// 代币符号（可选）
    pub token: Option<String>,
}

/// 检测响应
#[derive(Debug, Serialize)]
pub struct DetectionResponse {
    pub success: bool,
    pub data: Option<DetectionData>,
    pub error: Option<String>,
}

/// 检测数据
#[derive(Debug, Serialize)]
pub struct DetectionData {
    /// 是否检测到异常
    pub is_anomalous: bool,
    
    /// 异常分数 (0.0-1.0)
    pub score: f64,
    
    /// 威胁级别
    pub threat_level: String,
    
    /// 详细原因
    pub reason: String,
    
    /// 是否应该阻止
    pub should_block: bool,
    
    /// 关键因素
    pub key_factors: Vec<KeyFactor>,
    
    /// 详细信息（用于前端展示）
    pub details: DetectionDetails,
}

/// 关键因素
#[derive(Debug, Serialize)]
pub struct KeyFactor {
    pub name: String,
    pub contribution: f64,
}

/// 检测详细信息
#[derive(Debug, Serialize)]
pub struct DetectionDetails {
    /// 触发的规则列表
    pub triggered_rules: Vec<String>,
    
    /// 模型分数
    pub model_score: Option<f64>,
    
    /// 插件Warning
    pub plugin_alerts: Vec<String>,
    
    /// 建议操作
    pub recommended_action: String,
}

/// WebSocket事件消息
/// 
/// 符合前端期望的事件格式:
/// - type: 'transaction_blocked' | 'warning_issued' | 'detection_completed'
/// - message: 可选的消息描述
/// - threatLevel: 可选的威胁级别
/// - data: 可选的附加数据
#[derive(Debug, Clone, Serialize)]
pub struct AnomalyEventMessage {
    #[serde(rename = "type")]
    pub event_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none", rename = "threatLevel")]
    pub threat_level: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    
    pub timestamp: String,
}

/// 创建异常检测路由
pub fn create_anomaly_routes(state: AnomalyApiState) -> Router {
    Router::new()
        .route("/detect", post(detect_transaction))
        .route("/events", get(websocket_handler))
        .route("/config", get(get_config).post(update_config))
        .route("/stats", get(get_stats))
        .route("/history", get(get_history))
        .route("/simulate-events", post(simulate_events))
        .with_state(state)
}

/// POST /api/anomaly-detection/detect
/// 
/// 检测transaction是否存在异常
async fn detect_transaction(
    State(state): State<AnomalyApiState>,
    Json(req): Json<DetectionRequest>,
) -> Response {
    info!("收到检测请求: to={}, amount={}", req.to_address, req.amount);
    
    // fetch检测器
    let mut detector = state.detector.lock().await;
    
    // 执行检测
    let result = detector.detect_transaction(
        &req.to_address,
        req.amount,
        req.gas_price,
        req.is_contract,
    );
    
    // 解析原因，提取触发的规则
    let triggered_rules = extract_triggered_rules(&result.reason);
    let plugin_alerts = extract_plugin_alerts(&result.reason);
    
    // 构建响应
    let data = DetectionData {
        is_anomalous: result.is_anomalous,
        score: result.score,
        threat_level: format!("{:?}", result.threat_level),
        reason: result.reason.clone(),
        should_block: result.threat_level.should_block(),
        key_factors: result.key_factors.iter().map(|(name, contrib)| {
            KeyFactor {
                name: name.clone(),
                contribution: *contrib,
            }
        }).collect(),
        details: DetectionDetails {
            triggered_rules,
            model_score: if result.score > 0.0 { Some(result.score) } else { None },
            plugin_alerts,
            recommended_action: get_recommended_action(&result.threat_level),
        },
    };
    
    // 发送事件到WebSocket订阅者（符合前端期望的格式）
    let event_type = if data.should_block {
        "transaction_blocked"
    } else if data.is_anomalous {
        "warning_issued"
    } else {
        "detection_completed"
    };
    
    let event = AnomalyEventMessage {
        event_type: event_type.to_string(),
        message: Some(data.reason.clone()),
        threat_level: Some(data.threat_level.clone()),
        data: Some(serde_json::to_value(&data).unwrap_or_default()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let _ = state.event_tx.send(event);
    
    let response = DetectionResponse {
        success: true,
        data: Some(data),
        error: None,
    };
    
    Json(response).into_response()
}

/// WS /api/anomaly-detection/events
/// 
/// WebSocket连接，推送实时事件
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AnomalyApiState>,
) -> Response {
    info!("WebSocket连接请求");
    
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

/// 处理WebSocket连接
async fn handle_websocket(socket: WebSocket, state: AnomalyApiState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.event_tx.subscribe();
    
    info!("WebSocket已连接");
    
    // 发送欢迎消息
    let welcome = AnomalyEventMessage {
        event_type: "connected".to_string(),
        message: Some("Connected to anomaly detection event stream".to_string()),
        threat_level: None,
        data: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = sender.send(Message::Text(json)).await;
    }
    
    // 监听事件并转发
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });
    
    // 接收客户端消息（ping/pong）
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_data) => {
                    // 自动回复pong
                }
                _ => {}
            }
        }
    });
    
    // 等待任一任务completed
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
    
    info!("WebSocket已断开");
}

/// GET /api/anomaly-detection/config
/// 
/// fetch当前配置
async fn get_config(State(state): State<AnomalyApiState>) -> Response {
    let detector = state.detector.lock().await;
    let config = detector.config();
    
    Json(serde_json::json!({
        "success": true,
        "data": {
            "mode": format!("{:?}", detector.mode()),
            "model": {
                "enabled": config.model.enabled,
                "anomaly_threshold": config.model.anomaly_threshold,
            },
            "rule_engine": {
                "enabled": config.rule_engine.enabled,
                "high_value_threshold": config.rule_engine.high_value_threshold,
            },
            "feature_extraction": {
                "enabled": config.feature_extraction.enabled,
                "dust_threshold": config.feature_extraction.dust_threshold,
            },
            "events": {
                "enabled": config.events.enabled,
            },
            "storage": {
                "cache_size": config.storage.cache_size,
            }
        }
    })).into_response()
}

/// POST /api/anomaly-detection/config
/// 
/// 更新配置
async fn update_config(
    State(state): State<AnomalyApiState>,
    Json(req): Json<serde_json::Value>,
) -> Response {
    let mut detector = state.detector.lock().await;
    let mut config = detector.config().clone();
    
    // 更新配置字段
    if let Some(threshold) = req.get("anomaly_threshold").and_then(|v| v.as_f64()) {
        config.model.anomaly_threshold = threshold;
    }
    
    if let Some(high_value) = req.get("high_value_threshold").and_then(|v| v.as_f64()) {
        config.rule_engine.high_value_threshold = high_value;
    }
    
    // 应用配置
    if let Err(e) = detector.update_config(config) {
        return Json(serde_json::json!({
            "success": false,
            "error": e
        })).into_response();
    }
    
    Json(serde_json::json!({
        "success": true,
        "message": "配置已更新"
    })).into_response()
}

/// GET /api/anomaly-detection/stats
/// 
/// fetch统计信息
async fn get_stats(State(state): State<AnomalyApiState>) -> Response {
    let detector = state.detector.lock().await;
    let storage = detector.storage();
    
    let total = storage.count().unwrap_or(0);
    
    // query最近的记录
    let records = storage.query_records(None, None, 1000).unwrap_or_default();
    
    let mut blocked = 0;
    let mut warnings = 0;
    let mut passed = 0;
    let mut threat_dist = std::collections::HashMap::new();
    
    for record in &records {
        if record.result.is_anomalous {
            if record.result.threat_level.should_block() {
                blocked += 1;
            } else {
                warnings += 1;
            }
        } else {
            passed += 1;
        }
        
        let level = format!("{:?}", record.result.threat_level);
        *threat_dist.entry(level).or_insert(0) += 1;
    }
    
    Json(serde_json::json!({
        "success": true,
        "data": {
            "today": {
                "total_detections": total,
                "blocked": blocked,
                "warnings": warnings,
                "passed": passed
            },
            "threat_distribution": threat_dist
        }
    })).into_response()
}

/// GET /api/anomaly-detection/history
/// 
/// fetch检测历史
async fn get_history(State(state): State<AnomalyApiState>) -> Response {
    let detector = state.detector.lock().await;
    let storage = detector.storage();
    
    // query最近100条记录
    let records = storage.query_records(None, None, 100).unwrap_or_default();
    
    let history: Vec<_> = records.iter().map(|record| {
        serde_json::json!({
            "transaction_hash": record.transaction_hash,
            "timestamp": record.timestamp.to_rfc3339(),
            "is_anomalous": record.result.is_anomalous,
            "threat_level": format!("{:?}", record.result.threat_level),
            "score": record.result.score,
            "reason": record.result.reason,
            "blocked": record.result.threat_level.should_block(),
        })
    }).collect();
    
    Json(serde_json::json!({
        "success": true,
        "data": {
            "total": records.len(),
            "records": history
        }
    })).into_response()
}

// ========== 辅助函数 ==========

/// from原因文本中提取触发的规则
fn extract_triggered_rules(reason: &str) -> Vec<String> {
    let mut rules = Vec::new();
    
    if reason.contains("HIGH_VALUE") || reason.contains("高价值") {
        rules.push("高价值转账检测".to_string());
    }
    if reason.contains("BLACKLIST") || reason.contains("黑名单") {
        rules.push("黑名单address检测".to_string());
    }
    if reason.contains("NEW_ADDRESS") || reason.contains("新address") {
        rules.push("新addressWarning".to_string());
    }
    if reason.contains("DUST") || reason.contains("尘埃") {
        rules.push("尘埃攻击检测".to_string());
    }
    
    rules
}

/// from原因文本中提取插件Warning
fn extract_plugin_alerts(reason: &str) -> Vec<String> {
    let mut alerts = Vec::new();
    
    // 解析 "Plugin alerts: xxx" 部分
    if let Some(start) = reason.find("Plugin alerts:") {
        let plugin_part = &reason[start..];
        if let Some(end) = plugin_part.find(";") {
            let alert_text = &plugin_part[14..end].trim();
            alerts.push(alert_text.to_string());
        } else {
            let alert_text = plugin_part[14..].trim();
            alerts.push(alert_text.to_string());
        }
    }
    
    alerts
}

/// fetch推荐操作
fn get_recommended_action(threat_level: &ThreatLevel) -> String {
    match threat_level {
        ThreatLevel::None => "允许transaction".to_string(),
        ThreatLevel::Low => "允许transaction，记录日志".to_string(),
        ThreatLevel::Medium => "显示Warning，允许Continue".to_string(),
        ThreatLevel::High => "建议阻止，需要确认".to_string(),
        ThreatLevel::Critical => "立即阻止transaction".to_string(),
    }
}

/// POST /api/anomaly-detection/simulate-events
/// 
/// 模拟各种检测事件，用于前端测试
async fn simulate_events(
    State(state): State<AnomalyApiState>,
) -> Response {
    info!("模拟事件请求");
    
    // 模拟三种类型的事件
    let events = vec![
        // 1. transaction_blocked 事件
        AnomalyEventMessage {
            event_type: "transaction_blocked".to_string(),
            message: Some("检测到黑名单address，transaction已被阻止".to_string()),
            threat_level: Some("Critical".to_string()),
            data: Some(serde_json::json!({
                "is_anomalous": true,
                "score": 0.95,
                "threat_level": "Critical",
                "should_block": true,
                "reason": "目标address在黑名单中"
            })),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
        // 2. warning_issued 事件
        AnomalyEventMessage {
            event_type: "warning_issued".to_string(),
            message: Some("检测到高价值转账，please confirm carefully".to_string()),
            threat_level: Some("High".to_string()),
            data: Some(serde_json::json!({
                "is_anomalous": true,
                "score": 0.78,
                "threat_level": "High",
                "should_block": false,
                "reason": "转账金额超过高价值阈值"
            })),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
        // 3. detection_completed 事件
        AnomalyEventMessage {
            event_type: "detection_completed".to_string(),
            message: Some("transaction检测completed，未发现异常".to_string()),
            threat_level: Some("None".to_string()),
            data: Some(serde_json::json!({
                "is_anomalous": false,
                "score": 0.15,
                "threat_level": "None",
                "should_block": false,
                "reason": "No anomalies detected"
            })),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    ];
    
    // 发送所有模拟事件
    let mut sent_count = 0;
    for event in events {
        if state.event_tx.send(event).is_ok() {
            sent_count += 1;
        }
    }
    
    Json(serde_json::json!({
        "success": true,
        "message": format!("已发送 {} 个模拟事件", sent_count),
        "events": [
            "transaction_blocked (Critical)",
            "warning_issued (High)",
            "detection_completed (None)"
        ]
    })).into_response()
}

