#!/bin/bash
# 🔐 安全检查脚本 - 抓住 cargo audit 抓不到的逻辑错误

set -e

echo "🔐 运行自定义安全检查..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

ERRORS=0

# 1. 🔴 检查助记词派生是否使用空密码
echo ""
echo "📋 [1/5] 检查助记词派生安全性..."
if grep -rn 'to_seed("")' src/ 2>/dev/null; then
    echo "❌ 发现安全漏洞：助记词派生使用空密码"
    echo "   位置：src/core/"
    echo "   风险：允许仅凭助记词暴力破解私钥"
    echo "   修复：使用 to_seed(user_passphrase) 或 derive_master_key_secure()"
    ERRORS=$((ERRORS + 1))
else
    echo "✅ 助记词派生安全检查通过"
fi

# 2. 🔴 检查私钥是否强制 Zeroize（钱包命根子）
echo ""
echo "📋 [2/5] 检查私钥 Zeroize 强制使用（核心安全）..."

# 🔐 关键：检查 auth.rs、key_manager.rs、crypto/ 中的私钥处理
echo "  → 检查 auth.rs 种子解密..."
if grep -rn "decrypt.*seed\|derive.*key" src/auth/ 2>/dev/null | grep -v "Zeroizing\|SecretVec\|zeroize" | grep "\.rs:" || false; then
    echo "  ⚠️  auth 模块可能有未 zeroize 的敏感操作"
    ERRORS=$((ERRORS + 1))
fi

echo "  → 检查 key_manager.rs 私钥处理..."
UNSAFE_RETURNS=$(grep -rn "-> .*Vec<u8>" src/core/key_manager.rs src/core/wallet/ 2>/dev/null | \
    grep -i "key\|seed\|mnemonic" | \
    grep -v "Zeroizing\|SecretVec" | \
    grep -v "test\|//" || true)

if [ -n "$UNSAFE_RETURNS" ]; then
    echo "  ⚠️  发现可能未 zeroize 的敏感数据返回："
    echo "$UNSAFE_RETURNS"
    echo "     建议：使用 Zeroizing<Vec<u8>> 或 SecretVec"
    ERRORS=$((ERRORS + 1))
fi

echo "  → 检查 crypto 模块私钥擦除..."
if grep -rn "let.*key.*=.*\[" src/crypto/ 2>/dev/null | grep -v "zeroize\|Zeroizing" | grep "u8" | head -5 || false; then
    echo "  ⚠️  crypto 模块可能有未自动擦除的密钥数组"
fi

if [ $ERRORS -eq 0 ]; then
    echo "  ✅ 私钥 Zeroize 检查通过"
fi

# 3. 🔴 检查是否有明文私钥写入日志
echo ""
echo "📋 [3/5] 检查私钥日志泄漏..."

LEAKED_KEYS=$(grep -rn "info!\|debug!\|trace!\|println!" src/ 2>/dev/null | \
    grep -i "private.*key\|secret.*key\|mnemonic\|seed.*phrase" | \
    grep -v "//" | \
    grep -v "encrypted\|hash" || true)

if [ -n "$LEAKED_KEYS" ]; then
    echo "⚠️  发现可能的私钥日志泄漏："
    echo "$LEAKED_KEYS"
    echo "   风险：私钥可能被记录到日志文件"
    ERRORS=$((ERRORS + 1))
else
    echo "✅ 私钥日志检查通过"
fi

# 4. 🔴 检查签名验证是否有错误处理
echo ""
echo "📋 [4/5] 检查签名验证错误处理..."

UNSAFE_VERIFY=$(grep -rn "\.verify(" src/ 2>/dev/null | \
    grep -v "?" | \
    grep -v "unwrap\|expect\|match\|if let" | \
    grep "\.rs:" || true)

if [ -n "$UNSAFE_VERIFY" ]; then
    echo "⚠️  发现签名验证可能缺少错误处理："
    echo "$UNSAFE_VERIFY"
    ERRORS=$((ERRORS + 1))
else
    echo "✅ 签名验证检查通过"
fi

# 5. 🔴 检查是否使用了不安全的随机数生成
echo ""
echo "📋 [5/5] 检查随机数生成安全性..."

if grep -rn "rand::thread_rng\|StdRng\|SmallRng" src/crypto/ src/core/ 2>/dev/null | \
    grep -v "OsRng\|CryptoRng" | \
    grep "\.rs:" || true; then
    echo "⚠️  发现可能使用非密码学安全的RNG"
    echo "   建议：使用 OsRng 或实现 CryptoRng 的生成器"
else
    echo "✅ 随机数生成安全检查通过"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $ERRORS -gt 0 ]; then
    echo "❌ 发现 $ERRORS 个安全问题！"
    echo ""
    echo "🔴 这些是逻辑层面的安全问题，cargo audit 无法检测"
    echo "🔴 必须修复后才能合并到主分支"
    exit 1
else
    echo "✅ 所有自定义安全检查通过！"
    exit 0
fi

