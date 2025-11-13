#!/bin/bash

echo "🔐 安全启动服务器"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Step 1: 终止占用端口的进程..."
PIDS=$(netstat -ano | grep :8888 | awk '{print $5}' | sort -u)

if [ ! -z "$PIDS" ]; then
    echo "找到占用端口的进程: $PIDS"
    for pid in $PIDS; do
        echo "停止进程 $pid"
        taskkill //F //PID $pid 2>/dev/null || true
    done
else
    echo "没有找到占用端口的进程"
fi

echo ""
echo "Step 2: 等待端口释放..."
sleep 3

echo ""
echo "Step 3: 生成安全的WALLET_ENC_KEY..."
# 生成32字节随机密钥并base64编码
WALLET_ENC_KEY=$(openssl rand -base64 32)
echo "✅ 安全密钥已生成"

echo ""
echo "Step 4: 设置环境变量..."
export WALLET_ENC_KEY="$WALLET_ENC_KEY"
export API_KEY="testnet_api_key_117ca14556c34271"
export CORS_ALLOW_ORIGIN="http://localhost:3000"
export DATABASE_URL="sqlite://./data/testnet_wallet.db?mode=rwc"
export RUST_LOG="info"
export SERVER_HOST="127.0.0.1"
export SERVER_PORT="8888"

echo "✅ 环境变量已设置"
echo ""

echo "Step 5: 启动服务器..."
cargo run --bin hot_wallet
