#!/bin/bash
# 数据备份脚本

BACKUP_DIR="${BACKUP_DIR:-/var/backups/blockchain-wallet}"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

echo "开始备份..."

# 备份数据文件（如果使用SQLite）
if [ -f "wallet_data.db" ]; then
    cp wallet_data.db "$BACKUP_DIR/wallet_data_$DATE.db"
    echo "✅ 数据库已备份"
fi

# 备份配置
if [ -d "config" ]; then
    tar -czf "$BACKUP_DIR/config_$DATE.tar.gz" config/
    echo "✅ 配置已备份"
fi

# 清理30天前的备份
find "$BACKUP_DIR" -name "*.db" -mtime +30 -delete
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +30 -delete

echo "✅ 备份完成: $BACKUP_DIR"
ls -lh "$BACKUP_DIR" | tail -5

