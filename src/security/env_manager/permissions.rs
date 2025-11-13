//! 权限级别管理

/// 权限级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    /// 无权限
    None = 0,
    /// 只读权限
    ReadOnly = 1,
    /// 读写权限
    ReadWrite = 2,
    /// 管理员权限
    Admin = 3,
}

impl PermissionLevel {
    /// check是否有足够权限
    pub fn has_permission(&self, required: PermissionLevel) -> bool {
        *self >= required
    }

    /// from字符串解析权限级别
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(PermissionLevel::None),
            "readonly" | "read" => Some(PermissionLevel::ReadOnly),
            "readwrite" | "write" => Some(PermissionLevel::ReadWrite),
            "admin" => Some(PermissionLevel::Admin),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionLevel::None => "none",
            PermissionLevel::ReadOnly => "readonly",
            PermissionLevel::ReadWrite => "readwrite",
            PermissionLevel::Admin => "admin",
        }
    }
}

impl core::str::FromStr for PermissionLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(PermissionLevel::None),
            "readonly" | "read" => Ok(PermissionLevel::ReadOnly),
            "readwrite" | "write" => Ok(PermissionLevel::ReadWrite),
            "admin" => Ok(PermissionLevel::Admin),
            _ => Err(format!("invalid permission level: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_levels() {
        assert!(PermissionLevel::Admin > PermissionLevel::ReadWrite);
        assert!(PermissionLevel::ReadWrite > PermissionLevel::ReadOnly);
        assert!(PermissionLevel::ReadOnly > PermissionLevel::None);
    }

    #[test]
    fn test_has_permission() {
        assert!(PermissionLevel::Admin.has_permission(PermissionLevel::ReadOnly));
        assert!(PermissionLevel::ReadWrite.has_permission(PermissionLevel::ReadWrite));
        assert!(!PermissionLevel::ReadOnly.has_permission(PermissionLevel::Admin));
    }

    #[test]
    fn test_from_str() {
        assert_eq!(PermissionLevel::from_str("admin"), Some(PermissionLevel::Admin));
        assert_eq!(PermissionLevel::from_str("readonly"), Some(PermissionLevel::ReadOnly));
        assert_eq!(PermissionLevel::from_str("invalid"), None);
    }
}

