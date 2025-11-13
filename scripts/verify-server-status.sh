#!/bin/bash

echo "🔍 验证服务器运行状态"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 切换到项目目录
cd "$(dirname "$0")"

echo "Step 1: 检查端口8888状态..."
if netstat -ano | grep -q ":8888.*LISTENING"; then
    echo "✅ 端口8888正在监听"
    netstat -ano | grep ":8888.*LISTENING"
else
    echo "❌ 端口8888未在监听"
fi

echo ""
echo "Step 2: 测试健康检查..."
HEALTH_RESPONSE=$(curl -s -w "%{http_code}" http://localhost:8888/health 2>/dev/null)
HTTP_CODE="${HEALTH_RESPONSE: -3}"
RESPONSE_BODY="${HEALTH_RESPONSE%???}"

echo "HTTP状态码: $HTTP_CODE"
echo "响应内容: $RESPONSE_BODY"

if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ 健康检查成功"
else
    echo "❌ 健康检查失败"
fi

echo ""
echo "Step 3: 测试API端点..."
API_RESPONSE=$(curl -s -w "%{http_code}" -H "Authorization: Bearer testnet_api_key_117ca14556c34271" http://localhost:8888/wallets 2>/dev/null)
API_HTTP_CODE="${API_RESPONSE: -3}"
API_RESPONSE_BODY="${API_RESPONSE%???}"

echo "API HTTP状态码: $API_HTTP_CODE"
echo "API响应内容: $API_RESPONSE_BODY"

if [ "$API_HTTP_CODE" = "200" ]; then
    echo "✅ API端点正常"
else
    echo "❌ API端点异常"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 状态总结:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "$HTTP_CODE" = "200" ] && [ "$API_HTTP_CODE" = "200" ]; then
    echo "🎉 服务器运行正常！"
    echo ""
    echo "现在可以运行修复后的自动化测试："
    echo "./week_automated_test.sh"
else
    echo "⚠️ 服务器未运行或有问题"
    echo ""
    echo "请先启动服务器："
    echo "cargo run --bin hot_wallet"
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
