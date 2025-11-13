/*!
 * CSP (Content Security Policy) 中间件
 * 
 * 提供安全的内容安全策略，防止XSS攻击、点击劫持等安全威胁
 */
use axum::{
    body::Body,
    middleware::Next,
    response::Response,
    http::{Request, header::{HeaderValue, CONTENT_SECURITY_POLICY, X_FRAME_OPTIONS, X_CONTENT_TYPE_OPTIONS, HeaderName}},
};

/// CSP策略配置
#[derive(Debug, Clone)]
pub struct CspConfig {
    /// 开发模式（更宽松的策略）
    pub dev_mode: bool,
    /// API基础URL
    pub api_base_url: String,
    /// 前端URL
    pub frontend_url: String,
}

impl Default for CspConfig {
    fn default() -> Self {
        Self {
            dev_mode: cfg!(debug_assertions),
            api_base_url: "http://127.0.0.1:8080".to_string(),
            frontend_url: "http://localhost:3000".to_string(),
        }
    }
}

impl CspConfig {
    /// 生成CSP策略字符串
    pub fn generate_policy(&self) -> String {
        if self.dev_mode {
            // 开发模式：更宽松的策略，便于调试
            format!(
                "default-src 'self' {}; \
                 script-src 'self' 'unsafe-inline' 'unsafe-eval' {}; \
                 style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
                 font-src 'self' https://fonts.gstatic.com; \
                 img-src 'self' data: https:; \
                 connect-src 'self' {} ws://127.0.0.1:* ws://localhost:*; \
                 frame-ancestors 'self'; \
                 base-uri 'self'; \
                 form-action 'self';",
                self.frontend_url,
                self.frontend_url,
                self.api_base_url,
            )
        } else {
            // 生产模式：严格的CSP策略
            format!(
                "default-src 'self'; \
                 script-src 'self' 'sha256-{script_hash}'; \
                 style-src 'self' https://fonts.googleapis.com; \
                 font-src 'self' https://fonts.gstatic.com; \
                 img-src 'self' data: https:; \
                 connect-src 'self' {} wss://{}; \
                 frame-ancestors 'none'; \
                 base-uri 'self'; \
                 form-action 'self'; \
                 upgrade-insecure-requests; \
                 block-all-mixed-content;",
                self.api_base_url,
                self.api_base_url.replace("https://", ""),
                script_hash = "PLACEHOLDER_FOR_SCRIPT_HASH"
            )
        }
    }
}

/// CSP中间件
pub async fn csp_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    
    let headers = response.headers_mut();
    
    // fetch配置
    let config = CspConfig::default();
    let csp_policy = config.generate_policy();
    
    // 设置CSP头
    if let Ok(value) = HeaderValue::from_str(&csp_policy) {
        headers.insert(CONTENT_SECURITY_POLICY, value);
    }
    
    // 额外的安全头
    
    // X-Frame-Options: 防止点击劫持
    if let Ok(value) = HeaderValue::from_str("DENY") {
        headers.insert(X_FRAME_OPTIONS, value);
    }
    
    // X-Content-Type-Options: 防止MIME类型嗅探
    if let Ok(value) = HeaderValue::from_str("nosniff") {
        headers.insert(X_CONTENT_TYPE_OPTIONS, value);
    }
    
    // X-XSS-Protection: 启用XSS过滤器
    let name = HeaderName::from_static("x-xss-protection");
    if let Ok(value) = HeaderValue::from_str("1; mode=block") {
        headers.insert(name, value);
    }
    
    // Referrer-Policy: 控制Referer信息
    let name = HeaderName::from_static("referrer-policy");
    if let Ok(value) = HeaderValue::from_str("strict-origin-when-cross-origin") {
        headers.insert(name, value);
    }
    
    // Permissions-Policy: 限制浏览器功能
    let name = HeaderName::from_static("permissions-policy");
    if let Ok(value) = HeaderValue::from_str(
        "camera=(), microphone=(), geolocation=(), payment=()"
    ) {
        headers.insert(name, value);
    }
    
    // Strict-Transport-Security: 强制HTTPS (仅生产环境)
    if !config.dev_mode {
        let name = HeaderName::from_static("strict-transport-security");
        if let Ok(value) = HeaderValue::from_str("max-age=31536000; includeSubDomains; preload") {
            headers.insert(name, value);
        }
    }
    
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csp_config_default() {
        let config = CspConfig::default();
        let policy = config.generate_policy();
        
        assert!(policy.contains("default-src"));
        assert!(policy.contains("script-src"));
        assert!(policy.contains("connect-src"));
    }

    #[test]
    fn test_dev_mode_policy() {
        let config = CspConfig {
            dev_mode: true,
            ..Default::default()
        };
        let policy = config.generate_policy();
        
        // 开发模式应该允许unsafe-inline和unsafe-eval
        assert!(policy.contains("'unsafe-inline'"));
        assert!(policy.contains("'unsafe-eval'"));
    }

    #[test]
    fn test_prod_mode_policy() {
        let config = CspConfig {
            dev_mode: false,
            api_base_url: "https://api.example.com".to_string(),
            frontend_url: "https://app.example.com".to_string(),
        };
        let policy = config.generate_policy();
        
        // 生产模式应该更严格
        assert!(!policy.contains("'unsafe-inline'"));
        assert!(!policy.contains("'unsafe-eval'"));
        assert!(policy.contains("upgrade-insecure-requests"));
        assert!(policy.contains("block-all-mixed-content"));
    }
}

