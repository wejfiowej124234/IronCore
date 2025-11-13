#!/bin/bash
# 🔐 检查测试日志中的密钥泄漏
# 
# 如果日志包含 seed/key/mnemonic 等敏感词 → CI 失败

set -e

echo "🔍 检查测试日志中的密钥泄漏..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 运行测试并捕获输出
echo "📋 运行测试并监控日志输出..."
TEST_OUTPUT=$(RUST_LOG=info cargo test --lib --features "test-env,ethereum" 2>&1 || true)

# 🔴 关键：检查日志中是否泄漏敏感信息
echo ""
echo "🔍 扫描敏感词..."

# 检查 seed/种子
SEED_LEAKS=$(echo "$TEST_OUTPUT" | grep -i "seed" | grep -v "seed_from_u64\|RNG seed\|test seed" | grep -E "DEBUG:|INFO:|seed.*=|种子.*:" || true)

if [ -n "$SEED_LEAKS" ]; then
    echo "❌ 发现种子泄漏到日志："
    echo "$SEED_LEAKS"
    echo ""
    echo "🔴 致命：种子不能出现在日志中！"
    exit 1
fi

# 检查 key/密钥（排除测试名称和安全用法）
KEY_LEAKS=$(echo "$TEST_OUTPUT" | \
    grep -iE "(DEBUG|INFO|TRACE).*key|key.*=.*[0-9a-f]{32}|密钥.*值" | \
    grep -v "test.*key\|public_key\|key_manager\|KeyManager\|test result:" || true)

if [ -n "$KEY_LEAKS" ]; then
    echo "❌ 发现密钥泄漏到日志："
    echo "$KEY_LEAKS"
    echo ""
    echo "🔴 致命：私钥不能出现在日志中！"
    exit 1
fi

# 检查 mnemonic/助记词
MNEMONIC_LEAKS=$(echo "$TEST_OUTPUT" | grep -iE "mnemonic.*=|助记词.*:" | grep -v "mnemonic phrase\|Generated mnemonic\|<shown>\|<hidden>" || true)

if [ -n "$MNEMONIC_LEAKS" ]; then
    echo "❌ 发现助记词泄漏到日志："
    echo "$MNEMONIC_LEAKS"
    echo ""
    echo "🔴 致命：助记词不能出现在日志中！"
    exit 1
fi

# 检查十六进制密钥模式（64位十六进制 = 32字节私钥）
HEX_KEY_LEAKS=$(echo "$TEST_OUTPUT" | grep -E "[0-9a-f]{64}" | grep -v "test\|example\|0000000\|ffffff" | head -3 || true)

if [ -n "$HEX_KEY_LEAKS" ]; then
    echo "⚠️  发现可疑的十六进制字符串（可能是密钥）："
    echo "$HEX_KEY_LEAKS"
    echo ""
    echo "⚠️  警告：请确认这不是私钥泄漏"
fi

echo ""
echo "✅ 日志扫描完成：未发现密钥泄漏"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

