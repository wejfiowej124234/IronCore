-- 非托管钱包架构：添加钱包地址字段
-- 后端只存储公开的钱包地址，不存储私钥或助记词

-- 添加wallet_address字段（存储公钥地址）
ALTER TABLE user_wallets ADD COLUMN wallet_address TEXT;

-- 添加wallet_type字段（标准钱包/多签钱包等）
ALTER TABLE user_wallets ADD COLUMN wallet_type TEXT DEFAULT 'standard';

-- 添加索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_user_wallets_address ON user_wallets(wallet_address);

-- 注意：
-- wallet_name: 用户自定义的钱包名称（如"carl"）
-- wallet_address: 钱包的公开地址（如"0x123abc..."）
-- 不存储：private_key, mnemonic（这些由用户自己保管）

