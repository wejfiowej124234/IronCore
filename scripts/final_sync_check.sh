#!/bin/bash
# 最终同步检查 - GitHub仓库和本地文件一致性

cd "$(dirname "$0")"

echo "=========================================="
echo "Final Sync Check"
echo "GitHub Repository vs Local Files"
echo "=========================================="
echo ""

echo "[1/8] Fetching latest from GitHub..."
git fetch origin
echo "Done"
echo ""

echo "[2/8] Current branch:"
BRANCH=$(git branch --show-current)
echo "$BRANCH"
echo ""

echo "[3/8] Checking sync status..."
BEHIND=$(git rev-list --count HEAD..origin/main)
AHEAD=$(git rev-list --count origin/main..HEAD)
echo "Local behind GitHub: $BEHIND commits"
echo "Local ahead GitHub: $AHEAD commits"
echo ""

echo "[4/8] Checking working directory..."
UNCOMMITTED=$(git status -s | wc -l)
if [ $UNCOMMITTED -eq 0 ]; then
    echo "Working directory clean"
else
    echo "Uncommitted changes: $UNCOMMITTED files"
    git status -s
fi
echo ""

echo "[5/8] Latest commits comparison..."
echo "Local main latest 5 commits:"
git log --oneline -5
echo ""
echo "GitHub main latest 5 commits:"
git log origin/main --oneline -5
echo ""

echo "[6/8] File count comparison..."
LOCAL_FILES=$(git ls-files | wc -l)
REMOTE_FILES=$(git ls-tree -r origin/main --name-only | wc -l)
echo "Local tracked files: $LOCAL_FILES"
echo "GitHub tracked files: $REMOTE_FILES"
echo ""

echo "[7/8] Branch verification..."
BRANCHES=$(git branch -r | grep -v HEAD | wc -l)
echo "Remote branches: $BRANCHES"
git branch -r | grep -v HEAD
echo ""

echo "[8/8] Final verification..."
echo ""
if [ $BEHIND -eq 0 ] && [ $AHEAD -eq 0 ] && [ $UNCOMMITTED -eq 0 ]; then
    echo "=========================================="
    echo "PERFECT SYNC!"
    echo "=========================================="
    echo ""
    echo "Status:"
    echo "- Local and GitHub: IDENTICAL"
    echo "- No uncommitted changes"
    echo "- No pending commits"
    echo "- Files: $LOCAL_FILES (both local and remote)"
    echo ""
    echo "Your repository is perfectly synchronized!"
    echo ""
elif [ $UNCOMMITTED -gt 0 ]; then
    echo "=========================================="
    echo "ALMOST PERFECT"
    echo "=========================================="
    echo ""
    echo "Status:"
    echo "- Local and GitHub commits: SYNCED"
    echo "- Uncommitted changes: $UNCOMMITTED files"
    echo ""
    echo "Action needed:"
    echo "  git add ."
    echo "  git commit -m 'final cleanup'"
    echo "  git push origin main"
    echo ""
elif [ $AHEAD -gt 0 ]; then
    echo "=========================================="
    echo "NEED TO PUSH"
    echo "=========================================="
    echo ""
    echo "Status:"
    echo "- Local ahead: $AHEAD commits"
    echo ""
    echo "Action needed:"
    echo "  git push origin main"
    echo ""
elif [ $BEHIND -gt 0 ]; then
    echo "=========================================="
    echo "NEED TO PULL"
    echo "=========================================="
    echo ""
    echo "Status:"
    echo "- Local behind: $BEHIND commits"
    echo ""
    echo "Action needed:"
    echo "  git pull origin main"
    echo ""
fi

echo "GitHub repository:"
echo "https://github.com/Yinhang3377/Rust-Blockchain-Secure-Wallet"
echo ""
echo "Verification complete!"
echo "=========================================="

