#!/bin/bash

echo "🔍 检查测试网服务器状态"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 切换到项目目录
cd "$(dirname "$0")"

echo "Step 1: 检查端口8888状态..."
if netstat -ano | grep -q ":8888"; then
    echo "✅ 端口8888被占用（服务器可能正在运行）"
    netstat -ano | grep ":8888"
else
    echo "❌ 端口8888未被占用（服务器未运行）"
fi

echo ""
echo "Step 2: 测试健康检查..."
HEALTH_RESPONSE=$(curl -s -w "%{http_code}" http://localhost:8888/health 2>/dev/null)
HTTP_CODE="${HEALTH_RESPONSE: -3}"
RESPONSE_BODY="${HEALTH_RESPONSE%???}"

if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ 健康检查成功 (HTTP $HTTP_CODE)"
    echo "响应: $RESPONSE_BODY"
else
    echo "❌ 健康检查失败 (HTTP $HTTP_CODE)"
    echo "响应: $RESPONSE_BODY"
fi

echo ""
echo "Step 3: 检查环境变量文件..."
if [ -f ".env.testnet.local" ]; then
    echo "✅ 找到环境变量文件"
    echo "API密钥: $(grep "API_KEY=" .env.testnet.local | cut -d'=' -f2)"
else
    echo "❌ 未找到环境变量文件"
fi

echo ""
echo "Step 4: 检查数据目录..."
if [ -d "data" ]; then
    echo "✅ 数据目录存在"
    ls -la data/
else
    echo "❌ 数据目录不存在"
fi

echo ""
echo "Step 5: 检查进程..."
if pgrep -f hot_wallet >/dev/null; then
    echo "✅ 找到hot_wallet进程"
    pgrep -f hot_wallet
else
    echo "❌ 未找到hot_wallet进程"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 状态总结:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "$HTTP_CODE" = "200" ]; then
    echo "🎉 服务器运行正常！"
    echo ""
    echo "现在可以运行API测试："
    echo "curl -X POST http://localhost:8888/wallets \\"
    echo "  -H \"Authorization: Bearer \$(grep API_KEY= .env.testnet.local | cut -d'=' -f2)\" \\"
    echo "  -H \"Content-Type: application/json\" \\"
    echo "  -d '{\"name\": \"test_wallet\", \"description\": \"测试钱包\"}'"
else
    echo "⚠️ 服务器未运行或有问题"
    echo ""
    echo "请运行以下命令启动服务器："
    echo "cargo run --bin hot_wallet"
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
