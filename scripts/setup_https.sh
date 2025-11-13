#!/bin/bash
# HTTPS证书配置脚本（使用Let's Encrypt）

DOMAIN="${1:-wallet.example.com}"
EMAIL="${2:-admin@example.com}"

echo "🔒 配置HTTPS证书..."
echo "域名: $DOMAIN"
echo "邮箱: $EMAIL"
echo ""

# 检查certbot
if ! command -v certbot &> /dev/null; then
    echo "📦 安装certbot..."
    if command -v apt-get &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y certbot python3-certbot-nginx
    elif command -v yum &> /dev/null; then
        sudo yum install -y certbot python3-certbot-nginx
    else
        echo "❌ 请手动安装certbot"
        exit 1
    fi
fi

# 获取证书
echo "📜 获取SSL证书..."
sudo certbot --nginx -d "$DOMAIN" --email "$EMAIL" --agree-tos --non-interactive

# 设置自动续期
echo "🔄 设置自动续期..."
(crontab -l 2>/dev/null; echo "0 0 * * * certbot renew --quiet") | crontab -

echo ""
echo "✅ HTTPS配置完成！"
echo "证书位置: /etc/letsencrypt/live/$DOMAIN/"
echo "自动续期: 已配置"

