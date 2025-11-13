#!/bin/bash

echo "🔧 清理端口并启动正确的服务器"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 切换到项目目录
cd "$(dirname "$0")"

echo "Step 1: 停止占用8888端口的进程..."
# 停止进程ID 15152
taskkill //F //PID 15152 2>/dev/null || echo "进程已停止"

echo "Step 2: 等待端口释放..."
sleep 3

echo "Step 3: 验证端口已释放..."
if netstat -ano | grep -q ":8888.*LISTENING"; then
    echo "⚠️ 端口仍被占用，强制清理..."
    # 强制停止所有占用8888端口的进程
    for pid in $(netstat -ano | grep ":8888.*LISTENING" | awk '{print $5}' | sort -u); do
        if [ "$pid" != "0" ]; then
            echo "停止进程 $pid"
            taskkill //F //PID $pid 2>/dev/null || true
        fi
    done
    sleep 2
else
    echo "✅ 端口已释放"
fi

echo ""
echo "Step 4: 启动测试网服务器..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "服务器将在前台运行，按 Ctrl+C 停止"
echo "等待看到 'Server listening on 127.0.0.1:8888' 表示启动成功"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 启动服务器
cargo run --bin hot_wallet
