#!/bin/bash

# 🧹 最终仓库清理脚本
# 用途: 删除临时文件,整理结构,提交更改

set -e

echo "=========================================="
echo "🧹 开始最终仓库清理"
echo "=========================================="
echo ""

# 进入项目根目录
cd "$(dirname "$0")/.."

# 1. 删除临时和无效文件
echo "1️⃣ 删除临时和无效文件..."
echo ""

# 临时脚本
if [ -f "commit_rename.bat" ]; then
    echo "  ✅ 删除 commit_rename.bat"
    rm -f commit_rename.bat
fi

if [ -f "rename_to_english.sh" ]; then
    echo "  ✅ 删除 rename_to_english.sh"
    rm -f rename_to_english.sh
fi

# 无效文件
invalid_files=(
    "et --hard SHA_BEFORE"
    "CodeBAD_REQUEST -- src  Select-String -Pattern Unsupported chain # 定位剩余未改处"
    "h -u origin HEAD"
    "h origin main"
    "h origin main --force-with-lease"
    "Last"
    "Archive"
    "tatus --porcelain"
)

for file in "${invalid_files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ 删除 $file"
        rm -f "$file"
    fi
done

# 根目录的 mod.rs 和 utils.rs (应该在src中)
if [ -f "mod.rs" ] && [ -f "src/mod.rs" ]; then
    echo "  ✅ 删除根目录的 mod.rs (src中已有)"
    rm -f mod.rs
fi

if [ -f "utils.rs" ] && [ -f "src/utils.rs" ]; then
    echo "  ✅ 删除根目录的 utils.rs (src中已有)"
    rm -f utils.rs
fi

echo ""

# 2. 更新 README.md 链接
echo "2️⃣ 更新 README.md 链接..."
echo ""

if [ -f "README.md" ]; then
    # 备份原文件
    cp README.md README.md.bak
    
    # 更新GitHub用户名
    sed -i 's/wang-junxi3344-del/DarkCrab-Rust/g' README.md
    sed -i 's/Rust-Secure-Wallet-AI/Rust-Blockchain-Secure-Wallet/g' README.md
    
    echo "  ✅ README.md 链接已更新"
    echo "  📝 备份文件: README.md.bak"
else
    echo "  ⚠️  README.md 不存在"
fi

echo ""

# 3. 优化 .gitignore
echo "3️⃣ 优化 .gitignore..."
echo ""

cat >> .gitignore << 'EOF'

# macOS
.AppleDouble
.LSOverride

# Windows
Thumbs.db
ehthumbs.db
Desktop.ini

# Linux specific
.fuse_hidden*
.directory
.Trash-*

# Rust specific
**/*.rs.bk

# Scripts output
scripts/output/
scripts/logs/

# Temporary cleanup files
*.bak
README.md.bak
EOF

echo "  ✅ .gitignore 已优化"
echo ""

# 4. 创建 LICENSE 文件 (如果不存在)
echo "4️⃣ 检查 LICENSE 文件..."
echo ""

if [ ! -f "LICENSE" ]; then
    cat > LICENSE << 'EOF'
MIT License

Copyright (c) 2025 DarkCrab-Rust

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF
    echo "  ✅ LICENSE 文件已创建"
else
    echo "  ℹ️  LICENSE 文件已存在"
fi

echo ""

# 5. 显示清理结果
echo "=========================================="
echo "📊 清理结果统计"
echo "=========================================="
echo ""

# 统计删除的文件数
deleted_count=0
for file in "${invalid_files[@]}" "commit_rename.bat" "rename_to_english.sh"; do
    if [ ! -f "$file" ]; then
        ((deleted_count++))
    fi
done

echo "✅ 已删除临时文件: $deleted_count 个"
echo "✅ README.md 链接已更新"
echo "✅ .gitignore 已优化"
echo "✅ LICENSE 文件已检查"
echo ""

# 6. 显示 Git 状态
echo "=========================================="
echo "📋 当前 Git 状态"
echo "=========================================="
echo ""
git status --short

echo ""
echo "=========================================="
echo "🎯 下一步操作"
echo "=========================================="
echo ""
echo "1. 检查更改: git status"
echo "2. 查看差异: git diff"
echo "3. 提交更改:"
echo "   git add -A"
echo "   git commit -m 'chore: 最终仓库优化和清理'"
echo "4. 推送到GitHub:"
echo "   git push origin main"
echo ""
echo "=========================================="
echo "✅ 清理完成!"
echo "=========================================="

