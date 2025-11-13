-- 用户数据库 - 独立存储用户认证信息
-- 数据库文件: users.db
-- 安全措施: Argon2id密码哈希, 索引优化, 时间戳追踪

-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,                    -- UUID v4
    email TEXT UNIQUE NOT NULL,                       -- 邮箱（唯一）
    username TEXT,                                    -- 用户名（可选）
    password_hash TEXT NOT NULL,                      -- Argon2id密码哈希
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    last_login_at TEXT,                               -- 最后登录时间
    is_active INTEGER NOT NULL DEFAULT 1,             -- 账户是否激活
    failed_login_attempts INTEGER NOT NULL DEFAULT 0, -- 登录失败次数
    locked_until TEXT,                                -- 账户锁定到期时间
    
    -- 约束
    CHECK (email LIKE '%@%'),                         -- 简单的邮箱格式验证
    CHECK (length(password_hash) >= 64),              -- Argon2id哈希最小长度
    CHECK (is_active IN (0, 1)),                      -- 布尔值约束
    CHECK (failed_login_attempts >= 0)                -- 非负数
);

-- 用户-钱包关联表（关联到wallets.db）
CREATE TABLE IF NOT EXISTS user_wallets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,                            -- 用户ID
    wallet_name TEXT NOT NULL,                        -- 钱包名称（存储在wallets.db）
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    
    -- 约束
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE (user_id, wallet_name)                     -- 同一用户不能重复关联同一钱包
);

-- 会话/Token表
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    token TEXT UNIQUE NOT NULL,                       -- JWT或UUID token
    refresh_token TEXT UNIQUE,                        -- 刷新token
    expires_at TEXT NOT NULL,                         -- 过期时间
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    last_used_at TEXT,                                -- 最后使用时间
    ip_address TEXT,                                  -- 登录IP
    user_agent TEXT,                                  -- 用户代理
    is_revoked INTEGER NOT NULL DEFAULT 0,            -- 是否已撤销
    
    -- 约束
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CHECK (is_revoked IN (0, 1)),
    CHECK (expires_at > created_at)                   -- 过期时间必须晚于创建时间
);

-- 索引优化（提升查询性能）
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);

CREATE INDEX IF NOT EXISTS idx_user_wallets_user_id ON user_wallets(user_id);
CREATE INDEX IF NOT EXISTS idx_user_wallets_wallet_name ON user_wallets(wallet_name);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_sessions_is_revoked ON sessions(is_revoked);

-- 触发器：自动更新 updated_at
CREATE TRIGGER IF NOT EXISTS update_users_timestamp
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    UPDATE users SET updated_at = datetime('now', 'utc') WHERE id = NEW.id;
END;

-- Demo用户由代码自动创建（ensure_demo_user函数）
-- 不在SQL中预创建，避免密码哈希占位符问题

