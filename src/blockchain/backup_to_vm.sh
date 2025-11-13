#!/usr/bin/env bash
set -euo pipefail

stamp=$(date +%F_%H%M%S)
backup_dir="$(pwd)/../backups"
mkdir -p "$backup_dir"

echo "[1/6] Creating tar.gz backup..."
# 排除 backups 目录本身以避免递归
tar --exclude="$backup_dir" -czf "$backup_dir/defi-hot-wallet-backup-$stamp.tar.gz" ./

echo "[2/6] Creating git bundle (full repo)..."
git bundle create "$backup_dir/defi-hot-wallet-repo-$stamp.bundle" --all

echo "[3/6] Commit local changes if any..."
if ! git diff-index --quiet HEAD --; then
  git add -A
  git commit -m "WIP: local changes before VM test ($stamp)" || true
else
  echo "  no uncommitted changes"
fi

branch="local_test_backup_$stamp"
echo "[4/6] Creating branch $branch"
git branch -f "$branch"

echo "[5/6] Pushing branch and tag to origin (if remote configured)..."
git push origin "$branch":refs/heads/"$branch" || echo "  push branch failed or no origin"
tag="local_backup_$stamp"
git tag -f "$tag"
git push origin "$tag" || echo "  push tag failed or no origin"

# Optional: scp bundle to VM if VM_USER and VM_HOST are set
if [ -n "${VM_USER:-}" ] && [ -n "${VM_HOST:-}" ]; then
  dest="${VM_DEST:-/home/$VM_USER}"
  echo "[6/6] Copying bundle to $VM_USER@$VM_HOST:$dest"
  scp "$backup_dir/defi-hot-wallet-repo-$stamp.bundle" "$VM_USER@$VM_HOST:$dest" || echo "  scp failed"
fi

echo "Done. Backups in: $backup_dir"
echo "On VM you can restore with:"
echo "  scp user@host:/path/to/defi-hot-wallet-repo-$stamp.bundle ."
echo "  mkdir repo && cd repo"
echo "  git clone ../defi-hot-wallet-repo-$stamp.bundle ."
echo "  git checkout $branch"