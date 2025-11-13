param(
  [string]$VmUser = $env:VM_USER,
  [string]$VmHost = $env:VM_HOST,
  [string]$VmDest = $env:VM_DEST
)

$stamp = Get-Date -Format "yyyy-MM-dd_HHmmss"
$repoRoot = (Get-Location).Path
$backupDir = Join-Path $repoRoot "..\backups"
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

Write-Host "[1/6] Creating zip backup..."
$zipPath = Join-Path $backupDir "defi-hot-wallet-backup-$stamp.zip"
Compress-Archive -Path "$repoRoot\*" -DestinationPath $zipPath -Force

Write-Host "[2/6] Creating git bundle..."
$bundlePath = Join-Path $backupDir "defi-hot-wallet-repo-$stamp.bundle"
git bundle create $bundlePath --all

Write-Host "[3/6] Commit local changes if any..."
git diff-index --quiet HEAD --
if ($LASTEXITCODE -ne 0) {
    git add -A
    git commit -m "WIP: local changes before VM test ($stamp)" -q
} else {
    Write-Host "  no uncommitted changes"
}

$branch = "local_test_backup_$stamp"
Write-Host "[4/6] Creating branch $branch"
git branch -f $branch

Write-Host "[5/6] Pushing branch and tag to origin (if configured)..."
try { git push origin $branch:refs/heads/$branch -q } catch { Write-Host "  push branch failed or no origin" }
$tag = "local_backup_$stamp"
git tag -f $tag
try { git push origin $tag -q } catch { Write-Host "  push tag failed or no origin" }

if ($VmUser -and $VmHost) {
    $dest = if ($VmDest) { $VmDest } else { "/home/$VmUser" }
    Write-Host ("[6/6] Copying bundle to {0}@{1}:{2}" -f $VmUser, $VmHost, $dest)
    scp $bundlePath ("{0}@{1}:{2}" -f $VmUser, $VmHost, $dest)
}

Write-Host "Done. Backups in: $backupDir"
Write-Host "On VM to restore:"
Write-Host "  scp user@host:/path/to/defi-hot-wallet-repo-$stamp.bundle ."
Write-Host "  mkdir repo; cd repo"
Write-Host "  git clone ../defi-hot-wallet-repo-$stamp.bundle ."
Write-Host "  git checkout $branch"