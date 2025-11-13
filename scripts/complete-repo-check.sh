#!/bin/bash
# 完整检查GitHub仓库状态 - 解决PowerShell编码问题

cd "$(dirname "$0")"

echo "=========================================="
echo "完整检查GitHub仓库状态"
echo "=========================================="
echo ""

echo "[1/8] 检查远程仓库连接..."
git remote -v
echo ""

echo "[2/8] 获取最新远程信息..."
git fetch origin --prune
if [ $? -eq 0 ]; then
    echo "✅ 成功连接到GitHub"
else
    echo "❌ 无法连接到GitHub，请检查网络"
    exit 1
fi
echo ""

echo "[3/8] 查看所有远程分支..."
echo "远程分支列表:"
git branch -r
REMOTE_BRANCH_COUNT=$(git branch -r | wc -l)
echo ""
echo "远程分支总数: $REMOTE_BRANCH_COUNT"
echo ""

echo "[4/8] 查看本地分支..."
echo "本地分支列表:"
git branch -a
echo ""
echo "当前分支:"
CURRENT_BRANCH=$(git branch --show-current)
echo "$CURRENT_BRANCH"
echo ""

echo "[5/8] 检查本地和远程main分支的差异..."
git fetch origin main
echo ""
echo "本地main vs 远程main:"
git log HEAD..origin/main --oneline
if [ $? -eq 0 ]; then
    COMMITS_BEHIND=$(git log HEAD..origin/main --oneline | wc -l)
    if [ $COMMITS_BEHIND -eq 0 ]; then
        echo "✅ 本地代码是最新的"
    else
        echo "⚠️  远程有 $COMMITS_BEHIND 个新提交"
    fi
fi
echo ""

echo "[6/8] 查看远程main分支最近提交..."
echo "GitHub main分支最近10个提交:"
git log origin/main --oneline --decorate -10
echo ""

echo "[7/8] 查看是否有未推送的提交..."
git log origin/main..HEAD --oneline
COMMITS_AHEAD=$(git log origin/main..HEAD --oneline | wc -l)
if [ $COMMITS_AHEAD -eq 0 ]; then
    echo "✅ 没有未推送的提交"
else
    echo "📤 有 $COMMITS_AHEAD 个提交等待推送"
fi
echo ""

echo "[8/8] 检查工作区状态..."
git status --short
if [ $? -eq 0 ]; then
    CHANGED_FILES=$(git status --short | wc -l)
    if [ $CHANGED_FILES -eq 0 ]; then
        echo "✅ 工作区干净"
    else
        echo "📝 有 $CHANGED_FILES 个文件被修改"
    fi
fi
echo ""

echo "=========================================="
echo "仓库状态总结"
echo "=========================================="
echo ""
echo "仓库URL: https://github.com/Yinhang3377/Rust-Blockchain-Secure-Wallet"
echo "当前分支: $CURRENT_BRANCH"
echo "远程分支数: $REMOTE_BRANCH_COUNT"
echo "落后远程: $COMMITS_BEHIND 个提交"
echo "领先远程: $COMMITS_AHEAD 个提交"
echo "本地修改: $CHANGED_FILES 个文件"
echo ""

echo "=========================================="
echo "关于PR的说明"
echo "=========================================="
echo ""
echo "GitHub的PR（Pull Request）信息无法通过git命令直接获取。"
echo "PR是GitHub网页界面的功能，不是Git本身的功能。"
echo ""
echo "要查看PR，您必须："
echo "1. 在浏览器中访问: https://github.com/Yinhang3377/Rust-Blockchain-Secure-Wallet/pulls"
echo "2. 或使用GitHub CLI工具 (gh)"
echo ""

echo "如果您想使用GitHub CLI查看PR，请安装gh后运行:"
echo "  gh pr list --repo Yinhang3377/Rust-Blockchain-Secure-Wallet --state all"
echo ""

echo "=========================================="
echo "下一步建议"
echo "=========================================="
echo ""
if [ $CHANGED_FILES -gt 0 ]; then
    echo "您有本地修改，建议:"
    echo "  bash fix_and_commit.sh"
    echo ""
elif [ $COMMITS_AHEAD -gt 0 ]; then
    echo "您有未推送的提交，建议:"
    echo "  git push origin $CURRENT_BRANCH"
    echo ""
elif [ $COMMITS_BEHIND -gt 0 ]; then
    echo "远程有新提交，建议:"
    echo "  git pull origin $CURRENT_BRANCH"
    echo ""
else
    echo "✅ 本地和远程完全同步！"
    echo ""
fi

echo "=========================================="
echo "完成！"
echo "=========================================="

