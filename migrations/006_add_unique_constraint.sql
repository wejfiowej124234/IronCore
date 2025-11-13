-- 添加唯一约束：同一用户不能创建同名钱包
-- Migration: 006_add_unique_constraint
-- Date: 2025-11-05

-- 由于SQLite不支持直接添加UNIQUE约束到现有表，
-- 我们需要创建新表、复制数据、删除旧表、重命名新表

-- 1. 创建新表with UNIQUE约束
CREATE TABLE user_wallets_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    wallet_name TEXT NOT NULL,
    wallet_address TEXT,
    wallet_type TEXT NOT NULL DEFAULT 'standard',
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, wallet_name)  -- ✅ 唯一约束：同一用户不能有同名钱包
);

-- 2. 复制旧数据到新表
INSERT INTO user_wallets_new (id, user_id, wallet_name, wallet_address, wallet_type, created_at)
SELECT id, user_id, wallet_name, wallet_address, wallet_type, created_at
FROM user_wallets;

-- 3. 删除旧表
DROP TABLE user_wallets;

-- 4. 重命名新表
ALTER TABLE user_wallets_new RENAME TO user_wallets;

-- 5. 重建索引
CREATE INDEX IF NOT EXISTS idx_user_wallets_user_id ON user_wallets(user_id);
CREATE INDEX IF NOT EXISTS idx_user_wallets_wallet_address ON user_wallets(wallet_address);

