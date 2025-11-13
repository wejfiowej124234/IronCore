-- 创建用户偏好表
-- 用于存储用户的个性化设置（钱包选择、主题、语言等）

CREATE TABLE IF NOT EXISTS user_preferences (
    user_id TEXT PRIMARY KEY NOT NULL,
    last_selected_wallet TEXT,
    theme TEXT DEFAULT 'light' NOT NULL,
    language TEXT DEFAULT 'en-US' NOT NULL,
    notifications_enabled INTEGER DEFAULT 1 NOT NULL,
    two_fa_enabled INTEGER DEFAULT 0 NOT NULL,
    updated_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 创建索引以提升查询性能
CREATE INDEX IF NOT EXISTS idx_user_preferences_user_id ON user_preferences(user_id);
CREATE INDEX IF NOT EXISTS idx_user_preferences_updated_at ON user_preferences(updated_at);

-- 插入默认Demo用户的偏好（如果存在）
INSERT OR IGNORE INTO user_preferences (
    user_id,
    last_selected_wallet,
    theme,
    language,
    notifications_enabled,
    two_fa_enabled,
    updated_at,
    created_at
)
SELECT 
    id,
    NULL,
    'light',
    'en-US',
    1,
    0,
    strftime('%s', 'now'),
    strftime('%s', 'now')
FROM users
WHERE email = 'demo@securewallet.local';

