#!/bin/bash
# 生成安全密钥脚本

echo "🔐 生成生产环境密钥..."
echo ""

# 生成API Key
API_KEY=$(openssl rand -base64 32)
echo "API_KEY=$API_KEY"
echo ""

# 生成JWT Secret
JWT_SECRET=$(openssl rand -base64 64)
echo "JWT_SECRET=$JWT_SECRET"
echo ""

# 生成随机Session Secret
SESSION_SECRET=$(openssl rand -hex 32)
echo "SESSION_SECRET=$SESSION_SECRET"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⚠️  请将这些密钥保存到 .env.production"
echo "⚠️  不要提交到Git仓库！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 生成.env.production文件
cat > .env.production << EOF
# 自动生成的生产环境配置
# 生成时间: $(date)

API_KEY=$API_KEY
JWT_SECRET=$JWT_SECRET
DEV_MODE=0
RUST_LOG=info
CORS_ORIGIN=https://wallet.your-domain.com

# 请修改CORS_ORIGIN为您的实际域名
EOF

echo "✅ 已生成 .env.production"
echo "📝 请修改CORS_ORIGIN为您的实际域名"

