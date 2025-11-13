///! Cookie安全配置
///!
///! 提供安全的Cookie配置，包括SameSite属性

use axum::http::header::{HeaderValue, SET_COOKIE};
use axum::http::HeaderMap;

/// Cookie安全属性
#[derive(Debug, Clone)]
pub struct SecureCookieConfig {
    /// Cookie名称
    pub name: String,
    /// Cookie值
    pub value: String,
    /// 路径
    pub path: String,
    /// 最大年龄（秒）
    pub max_age: Option<i64>,
    /// HttpOnly标志
    pub http_only: bool,
    /// Secure标志（HTTPS only）
    pub secure: bool,
    /// SameSite属性
    pub same_site: SameSitePolicy,
}

/// SameSite策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSitePolicy {
    /// Strict: 完全禁止第三方Cookie
    Strict,
    /// Lax: 允许安全的跨站请求（GET）
    Lax,
    /// None: 允许所有跨站请求（需要Secure）
    None,
}

impl SameSitePolicy {
    fn as_str(&self) -> &'static str {
        match self {
            SameSitePolicy::Strict => "Strict",
            SameSitePolicy::Lax => "Lax",
            SameSitePolicy::None => "None",
        }
    }
}

impl Default for SecureCookieConfig {
    fn default() -> Self {
        Self {
            name: "session".to_string(),
            value: String::new(),
            path: "/".to_string(),
            max_age: Some(3600), // 1小时
            http_only: true,
            secure: true,
            same_site: SameSitePolicy::Strict,  // ✅ 默认使用Strict
        }
    }
}

impl SecureCookieConfig {
    /// 创建新的Cookie配置
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            ..Default::default()
        }
    }

    /// 设置SameSite策略
    pub fn with_same_site(mut self, policy: SameSitePolicy) -> Self {
        self.same_site = policy;
        self
    }

    /// 设置最大年龄
    pub fn with_max_age(mut self, max_age: i64) -> Self {
        self.max_age = Some(max_age);
        self
    }

    /// 设置HttpOnly
    pub fn with_http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    /// 设置Secure
    pub fn with_secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// 设置路径
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// 构建Cookie字符串
    pub fn build(&self) -> String {
        let mut cookie = format!("{}={}; Path={}", self.name, self.value, self.path);

        if let Some(max_age) = self.max_age {
            cookie.push_str(&format!("; Max-Age={}", max_age));
        }

        if self.http_only {
            cookie.push_str("; HttpOnly");
        }

        if self.secure {
            cookie.push_str("; Secure");
        }

        // ✅ 添加SameSite属性
        cookie.push_str(&format!("; SameSite={}", self.same_site.as_str()));

        cookie
    }

    /// 添加到HTTP响应头
    pub fn add_to_headers(&self, headers: &mut HeaderMap) -> Result<(), String> {
        let cookie_str = self.build();
        let header_value = HeaderValue::from_str(&cookie_str)
            .map_err(|e| format!("Invalid cookie value: {}", e))?;
        
        headers.insert(SET_COOKIE, header_value);
        Ok(())
    }
}

/// 创建安全的会话Cookie
///
/// # Arguments
/// * `session_token` - 会话令牌
/// * `max_age` - Cookie有效期（秒）
/// * `is_production` - 是否为生产环境
///
/// # Returns
/// 配置好的Cookie
pub fn create_session_cookie(
    session_token: &str,
    max_age: i64,
    is_production: bool,
) -> SecureCookieConfig {
    SecureCookieConfig::new("session_token", session_token)
        .with_max_age(max_age)
        .with_http_only(true)
        .with_secure(is_production)  // 生产环境必须HTTPS
        .with_same_site(SameSitePolicy::Strict)  // ✅ 防止CSRF
        .with_path("/")
}

/// 创建安全的认证Cookie
///
/// # Arguments
/// * `access_token` - Access token
/// * `max_age` - Cookie有效期（秒）
///
/// # Returns
/// 配置好的Cookie
pub fn create_auth_cookie(
    access_token: &str,
    max_age: i64,
) -> SecureCookieConfig {
    // from环境变量判断是否为生产环境
    let is_production = std::env::var("RUST_ENV")
        .unwrap_or_else(|_| "development".to_string()) == "production";

    SecureCookieConfig::new("auth_token", access_token)
        .with_max_age(max_age)
        .with_http_only(true)
        .with_secure(is_production)
        .with_same_site(SameSitePolicy::Lax)  // Lax允许from外部链接登录
        .with_path("/api")
}

/// 创建CookieDelete指令（用于登出）
///
/// # Arguments
/// * `cookie_name` - Cookie名称
///
/// # Returns
/// 过期的Cookie配置
pub fn create_delete_cookie(cookie_name: &str) -> SecureCookieConfig {
    SecureCookieConfig::new(cookie_name, "")
        .with_max_age(0)  // 立即过期
        .with_http_only(true)
        .with_secure(true)
        .with_same_site(SameSitePolicy::Strict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_build_with_samesite() {
        let cookie = SecureCookieConfig::new("test", "value")
            .with_same_site(SameSitePolicy::Strict)
            .build();
        
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
    }

    #[test]
    fn test_session_cookie_creation() {
        let cookie = create_session_cookie("test_token", 3600, true);
        let cookie_str = cookie.build();
        
        assert!(cookie_str.contains("session_token=test_token"));
        assert!(cookie_str.contains("Max-Age=3600"));
        assert!(cookie_str.contains("HttpOnly"));
        assert!(cookie_str.contains("Secure"));
        assert!(cookie_str.contains("SameSite=Strict"));
    }

    #[test]
    fn test_delete_cookie() {
        let cookie = create_delete_cookie("test_cookie");
        let cookie_str = cookie.build();
        
        assert!(cookie_str.contains("Max-Age=0"));
        assert!(cookie_str.contains("SameSite=Strict"));
    }

    #[test]
    fn test_samesite_policies() {
        assert_eq!(SameSitePolicy::Strict.as_str(), "Strict");
        assert_eq!(SameSitePolicy::Lax.as_str(), "Lax");
        assert_eq!(SameSitePolicy::None.as_str(), "None");
    }
}

