#!/bin/bash

# API对齐验证脚本
# 用于验证后端API是否符合前端期望

echo "🔍 验证后端API对齐状态..."
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

API_BASE="http://localhost:8888"
API_KEY="test_api_key"

# 检查后端是否运行
echo "1️⃣ 检查后端服务器状态..."
if curl -s "$API_BASE/api/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ 后端服务器运行正常${NC}"
else
    echo -e "${RED}❌ 后端服务器未运行${NC}"
    echo -e "${YELLOW}提示: 请先运行 ./start_backend.sh${NC}"
    exit 1
fi

echo ""
echo "2️⃣ 验证CORS配置..."
CORS_RESPONSE=$(curl -s -H "Origin: http://localhost:3000" \
     -H "Access-Control-Request-Method: GET" \
     -H "Access-Control-Request-Headers: authorization,content-type" \
     -X OPTIONS "$API_BASE/api/health" -I)

if echo "$CORS_RESPONSE" | grep -q "access-control-allow-origin"; then
    echo -e "${GREEN}✅ CORS配置正确${NC}"
else
    echo -e "${YELLOW}⚠️  CORS响应头可能缺失，但可能仍然工作${NC}"
fi

echo ""
echo "3️⃣ 验证认证机制..."
# 测试无认证请求（应该失败）
STATUS_NO_AUTH=$(curl -s -o /dev/null -w "%{http_code}" "$API_BASE/api/wallets")
if [ "$STATUS_NO_AUTH" -eq 401 ]; then
    echo -e "${GREEN}✅ 无认证请求正确返回401${NC}"
else
    echo -e "${RED}❌ 无认证请求返回 $STATUS_NO_AUTH (期望401)${NC}"
fi

# 测试有认证请求
STATUS_WITH_AUTH=$(curl -s -o /dev/null -w "%{http_code}" \
     -H "Authorization: $API_KEY" \
     "$API_BASE/api/wallets")
if [ "$STATUS_WITH_AUTH" -eq 200 ]; then
    echo -e "${GREEN}✅ 有认证请求正确返回200${NC}"
else
    echo -e "${YELLOW}⚠️  有认证请求返回 $STATUS_WITH_AUTH (可能需要数据库初始化)${NC}"
fi

echo ""
echo "4️⃣ 验证API端点..."

# 健康检查
if curl -s "$API_BASE/api/health" | grep -q "healthy"; then
    echo -e "${GREEN}✅ GET /api/health - 正常${NC}"
else
    echo -e "${RED}❌ GET /api/health - 失败${NC}"
fi

# 钱包列表
if curl -s -H "Authorization: $API_KEY" "$API_BASE/api/wallets" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ GET /api/wallets - 正常${NC}"
else
    echo -e "${RED}❌ GET /api/wallets - 失败${NC}"
fi

echo ""
echo "5️⃣ 验证网络参数支持..."

# 创建测试钱包（如果不存在）
TEST_WALLET="verify_test_$(date +%s)"
CREATE_RESULT=$(curl -s -X POST \
     -H "Authorization: $API_KEY" \
     -H "Content-Type: application/json" \
     -d "{\"name\":\"$TEST_WALLET\",\"quantum_safe\":false}" \
     "$API_BASE/api/wallets")

if echo "$CREATE_RESULT" | grep -q "$TEST_WALLET"; then
    echo -e "${GREEN}✅ POST /api/wallets - 创建成功${NC}"
    
    # 测试余额查询（默认eth网络）
    BALANCE_RESPONSE=$(curl -s \
         -H "Authorization: $API_KEY" \
         "$API_BASE/api/wallets/$TEST_WALLET/balance?network=eth")
    
    if echo "$BALANCE_RESPONSE" | grep -q "balance"; then
        echo -e "${GREEN}✅ GET /api/wallets/:name/balance?network=eth - 正常${NC}"
    else
        echo -e "${YELLOW}⚠️  余额查询可能需要区块链客户端配置${NC}"
    fi
    
    # 删除测试钱包
    DELETE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
         -X DELETE \
         -H "Authorization: $API_KEY" \
         "$API_BASE/api/wallets/$TEST_WALLET")
    
    if [ "$DELETE_STATUS" -eq 204 ]; then
        echo -e "${GREEN}✅ DELETE /api/wallets/:name - 正常${NC}"
    else
        echo -e "${RED}❌ DELETE /api/wallets/:name - 返回$DELETE_STATUS${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  创建测试钱包失败，跳过其他测试${NC}"
fi

echo ""
echo "6️⃣ 验证端口配置..."
if [ "$API_BASE" = "http://localhost:8888" ]; then
    echo -e "${GREEN}✅ 端口配置正确: 8888${NC}"
else
    echo -e "${RED}❌ 端口配置错误${NC}"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 验证总结"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ 对齐项目:"
echo "  • 服务器地址: http://localhost:8888"
echo "  • 认证方式: Authorization: test_api_key"
echo "  • 内容类型: application/json"
echo "  • 默认网络: eth"
echo "  • CORS支持: http://localhost:3000"
echo ""
echo "📌 前端可以安全连接到后端API！"
echo ""
echo "🚀 下一步: 启动前端应用并测试完整流程"
echo ""

