#!/bin/bash
# 健康检查脚本

API_URL="${API_URL:-http://127.0.0.1:8080}"

echo "检查后端健康状态..."
response=$(curl -s -w "\n%{http_code}" "${API_URL}/api/health")
http_code=$(echo "$response" | tail -n1)
body=$(echo "$response" | head -n-1)

if [ "$http_code" = "200" ]; then
    echo "✅ 后端健康"
    echo "$body"
    exit 0
else
    echo "❌ 后端异常 (HTTP $http_code)"
    echo "$body"
    exit 1
fi

