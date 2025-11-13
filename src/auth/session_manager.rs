//! 会话管理模块
//!
//! 提供安全的会话管理，包括Token过期、撤销等

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Access Token
    pub access_token: String,
    /// Refresh Token
    pub refresh_token: Option<String>,
    /// Creation time
    pub created_at: Instant,
    /// 最后活动时间
    pub last_activity: Instant,
    /// 过期时间
    pub expires_at: Instant,
    /// 来源IP
    pub ip_address: Option<String>,
    /// User Agent
    pub user_agent: Option<String>,
}

/// 会话管理器
pub struct SessionManager {
    /// 活跃会话存储 (session_id -> Session)
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Token到Session的映射 (access_token -> session_id)
    token_to_session: Arc<RwLock<HashMap<String, String>>>,
    /// 会话超时时间
    session_timeout: Duration,
    /// 最大并发会话数（每user）
    max_sessions_per_user: usize,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        Self {
            // ✅ 预分配容量（预估5000个并发会话）
            sessions: Arc::new(RwLock::new(HashMap::with_capacity(5000))),
            token_to_session: Arc::new(RwLock::new(HashMap::with_capacity(5000))),
            session_timeout: Duration::from_secs(3600),  // 1小时
            max_sessions_per_user: 5,  // 每user最多5个会话
        }
    }
    
    /// 创建新会话
    ///
    /// # Arguments
    /// * `user_id` - User ID
    /// * `access_token` - Access token
    /// * `refresh_token` - Refresh token（可选）
    /// * `ip_address` - 来源IP
    /// * `user_agent` - User Agent
    pub fn create_session(
        &self,
        user_id: &str,
        access_token: String,
        refresh_token: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> String {
        let session_id = Uuid::new_v4().to_string();
        let now = Instant::now();
        
        let session = Session {
            id: session_id.clone(),
            user_id: user_id.to_string(),
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
            created_at: now,
            last_activity: now,
            expires_at: now + self.session_timeout,
            ip_address,
            user_agent,
        };
        
        // 存储会话
        {
            let mut sessions = self.sessions.write();
            
            // check并限制每user的会话数
            let user_sessions: Vec<_> = sessions.iter()
                .filter(|(_, s)| s.user_id == user_id)
                .map(|(id, _)| id.clone())
                .collect();
            
            if user_sessions.len() >= self.max_sessions_per_user {
                // 移除最老的会话
                if let Some(oldest_id) = user_sessions.first() {
                    sessions.remove(oldest_id);
                    // 同时清理token映射
                    self.token_to_session.write().retain(|_, sid| sid != oldest_id);
                }
            }
            
            sessions.insert(session_id.clone(), session);
        }
        
        // 建立token到session的映射
        {
            let mut token_map = self.token_to_session.write();
            token_map.insert(access_token, session_id.clone());
            if let Some(ref rt) = refresh_token {
                token_map.insert(rt.clone(), session_id.clone());
            }
        }
        
        session_id
    }
    
    /// validate并更新会话
    ///
    /// # Returns
    /// Some(user_id) if valid, None if invalid/expired
    pub fn validate_and_refresh(&self, access_token: &str) -> Option<String> {
        // fetchsession_id
        let session_id = {
            let token_map = self.token_to_session.read();
            token_map.get(access_token).cloned()?
        };
        
        // fetch并validatesession
        let mut sessions = self.sessions.write();
        let session = sessions.get_mut(&session_id)?;
        
        let now = Instant::now();
        
        // check是否过期
        if now > session.expires_at {
            // 过期，移除会话
            sessions.remove(&session_id);
            self.token_to_session.write().remove(access_token);
            return None;
        }
        
        // 更新最后活动时间（滑动过期）
        session.last_activity = now;
        session.expires_at = now + self.session_timeout;
        
        Some(session.user_id.clone())
    }
    
    /// 撤销会话（登出）
    pub fn revoke_session(&self, access_token: &str) -> bool {
        // fetchsession_id
        let session_id = {
            let token_map = self.token_to_session.read();
            match token_map.get(access_token) {
                Some(id) => id.clone(),
                None => return false,
            }
        };
        
        // 移除会话
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.remove(&session_id) {
            // 清理所有相关token
            let mut token_map = self.token_to_session.write();
            token_map.remove(&session.access_token);
            if let Some(rt) = session.refresh_token {
                token_map.remove(&rt);
            }
            true
        } else {
            false
        }
    }
    
    /// 撤销user的所有会话
    pub fn revoke_all_user_sessions(&self, user_id: &str) -> usize {
        let mut sessions = self.sessions.write();
        let mut token_map = self.token_to_session.write();
        
        // 找到user的所有会话
        let user_session_ids: Vec<_> = sessions.iter()
            .filter(|(_, s)| s.user_id == user_id)
            .map(|(id, _)| id.clone())
            .collect();
        
        let count = user_session_ids.len();
        
        // 移除会话和token映射
        for session_id in user_session_ids {
            if let Some(session) = sessions.remove(&session_id) {
                token_map.remove(&session.access_token);
                if let Some(rt) = session.refresh_token {
                    token_map.remove(&rt);
                }
            }
        }
        
        count
    }
    
    /// fetch活跃会话数
    pub fn active_session_count(&self) -> usize {
        self.sessions.read().len()
    }
    
    /// 清理过期会话（定期任务）
    pub fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write();
        let mut token_map = self.token_to_session.write();
        let now = Instant::now();
        
        let expired: Vec<_> = sessions.iter()
            .filter(|(_, s)| now > s.expires_at)
            .map(|(id, _)| id.clone())
            .collect();
        
        let count = expired.len();
        
        for session_id in expired {
            if let Some(session) = sessions.remove(&session_id) {
                token_map.remove(&session.access_token);
                if let Some(rt) = session.refresh_token {
                    token_map.remove(&rt);
                }
            }
        }
        
        count
    }
    
    /// 启动定期清理任务
    pub async fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                let cleaned = self.cleanup_expired_sessions();
                if cleaned > 0 {
                    tracing::info!("Session cleanup: removed {} expired sessions", cleaned);
                }
            }
        });
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_creation() {
        let manager = SessionManager::new();
        let session_id = manager.create_session(
            "user123",
            "token_abc".to_string(),
            None,
            Some("192.168.1.1".to_string()),
            None,
        );
        
        assert!(!session_id.is_empty());
    }
    
    #[test]
    fn test_session_validation() {
        let manager = SessionManager::new();
        let token = "token_xyz".to_string();
        
        manager.create_session("user123", token.clone(), None, None, None);
        
        // 应该validate通过
        assert_eq!(manager.validate_and_refresh(&token), Some("user123".to_string()));
        
        // 无效token应该failed
        assert_eq!(manager.validate_and_refresh("invalid"), None);
    }
    
    #[test]
    fn test_session_revocation() {
        let manager = SessionManager::new();
        let token = "token_abc".to_string();
        
        manager.create_session("user123", token.clone(), None, None, None);
        
        // 撤销
        assert!(manager.revoke_session(&token));
        
        // 撤销后应该无效
        assert_eq!(manager.validate_and_refresh(&token), None);
    }
    
    #[test]
    fn test_max_sessions_per_user() {
        let manager = SessionManager::new();
        let user_id = "user123";
        
        // 创建6个会话（超过限制5个）
        for i in 0..6 {
            manager.create_session(
                user_id,
                format!("token_{}", i),
                None,
                None,
                None,
            );
        }
        
        // 应该只保留最新的5个
        let sessions = manager.sessions.read();
        let user_sessions = sessions.iter()
            .filter(|(_, s)| s.user_id == user_id)
            .count();
        
        assert_eq!(user_sessions, 5);
    }
}

