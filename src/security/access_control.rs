// src/security/access_control.rs
use crate::core::errors::WalletError;
use std::collections::HashMap;

/// Role definitions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    User,
    Auditor,
    Guest,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::User => write!(f, "user"),
            Role::Auditor => write!(f, "auditor"),
            Role::Guest => write!(f, "guest"),
        }
    }
}

/// Permission definitions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    CreateWallet,
    TransferFunds,
    ViewBalance,
    AuditLogs,
    ManageUsers,
    SystemConfig,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Permission::CreateWallet => write!(f, "create_wallet"),
            Permission::TransferFunds => write!(f, "transfer_funds"),
            Permission::ViewBalance => write!(f, "view_balance"),
            Permission::AuditLogs => write!(f, "audit_logs"),
            Permission::ManageUsers => write!(f, "manage_users"),
            Permission::SystemConfig => write!(f, "system_config"),
        }
    }
}

/// Access control manager
pub struct AccessControl {
    user_roles: HashMap<String, Vec<Role>>,
    role_permissions: HashMap<Role, Vec<Permission>>,
}

impl AccessControl {
    /// Create a new access control manager with default role-permission mapping.
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        // Define permissions for roles
        role_permissions.insert(
            Role::Admin,
            vec![
                Permission::CreateWallet,
                Permission::TransferFunds,
                Permission::ViewBalance,
                Permission::AuditLogs,
                Permission::ManageUsers,
                Permission::SystemConfig,
            ],
        );

        role_permissions.insert(
            Role::User,
            vec![Permission::CreateWallet, Permission::TransferFunds, Permission::ViewBalance],
        );

        role_permissions
            .insert(Role::Auditor, vec![Permission::ViewBalance, Permission::AuditLogs]);

        role_permissions.insert(Role::Guest, vec![Permission::ViewBalance]);

        Self { user_roles: HashMap::new(), role_permissions }
    }

    /// Assign a role to a user.
    pub fn assign_role(&mut self, user_id: &str, role: Role) -> Result<(), WalletError> {
        self.user_roles.entry(user_id.to_string()).or_default().push(role);
        Ok(())
    }

    /// Revoke a role from a user.
    pub fn revoke_role(&mut self, user_id: &str, role: &Role) -> Result<(), WalletError> {
        if let Some(roles) = self.user_roles.get_mut(user_id) {
            roles.retain(|r| r != role);
        }
        Ok(())
    }

    /// Check whether a user has a specific role.
    pub fn has_role(&self, user_id: &str, role: &Role) -> bool {
        self.user_roles.get(user_id).map(|roles| roles.contains(role)).unwrap_or(false)
    }

    /// Check whether a user has a specific permission (via assigned roles).
    pub fn has_permission(&self, user_id: &str, permission: &Permission) -> bool {
        if let Some(user_roles) = self.user_roles.get(user_id) {
            for role in user_roles {
                if let Some(role_permissions) = self.role_permissions.get(role) {
                    if role_permissions.contains(permission) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get roles assigned to a user.
    pub fn get_user_roles(&self, user_id: &str) -> Vec<Role> {
        self.user_roles.get(user_id).cloned().unwrap_or_default()
    }

    /// Get permissions associated with a role.
    pub fn get_role_permissions(&self, role: &Role) -> Vec<Permission> {
        self.role_permissions.get(role).cloned().unwrap_or_default()
    }

    /// Check whether a user is an admin.
    pub fn is_admin(&self, user_id: &str) -> bool {
        self.has_role(user_id, &Role::Admin)
    }

    /// Check whether a user is an auditor.
    pub fn is_auditor(&self, user_id: &str) -> bool {
        self.has_role(user_id, &Role::Auditor)
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_assignment() {
        let mut ac = AccessControl::new();
        let user_id = "user123";

        // assign role
        ac.assign_role(user_id, Role::User).unwrap();
        assert!(ac.has_role(user_id, &Role::User));

        // permission checks
        assert!(ac.has_permission(user_id, &Permission::CreateWallet));
        assert!(ac.has_permission(user_id, &Permission::ViewBalance));
        assert!(!ac.has_permission(user_id, &Permission::AuditLogs));
    }

    #[test]
    fn test_role_revocation() {
        let mut ac = AccessControl::new();
        let user_id = "user123";

        // assign then revoke admin role
        ac.assign_role(user_id, Role::Admin).unwrap();
        assert!(ac.has_role(user_id, &Role::Admin));

        ac.revoke_role(user_id, &Role::Admin).unwrap();
        assert!(!ac.has_role(user_id, &Role::Admin));
    }

    #[test]
    fn test_permission_check() {
        let mut ac = AccessControl::new();
        let user_id = "user123";

        ac.assign_role(user_id, Role::Auditor).unwrap();

        assert!(ac.has_permission(user_id, &Permission::ViewBalance));
        assert!(ac.has_permission(user_id, &Permission::AuditLogs));
        assert!(!ac.has_permission(user_id, &Permission::ManageUsers));
    }

    #[test]
    fn test_admin_check() {
        let mut ac = AccessControl::new();
        let admin_id = "admin123";
        let user_id = "user123";

        ac.assign_role(admin_id, Role::Admin).unwrap();
        ac.assign_role(user_id, Role::User).unwrap();

        assert!(ac.is_admin(admin_id));
        assert!(!ac.is_admin(user_id));
    }
}
