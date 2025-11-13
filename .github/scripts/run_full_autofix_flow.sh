#!/usr/bin/env bash
# run_full_autofix_flow.sh
# Usage: export GITHUB_PAT="ghp_..." && bash .github/scripts/run_full_autofix_flow.sh

set -euo pipefail

# Prefer bundled jq if available (helps on Windows without admin install)
if [ -x ".github/bin/jq.exe" ]; then
  export PATH="$PWD/.github/bin:$PATH"
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "This script requires 'jq' to be installed." >&2
  exit 1
fi
if ! command -v git >/dev/null 2>&1; then
  echo "This script requires 'git' to be installed." >&2
  exit 1
fi

GITHUB_PAT=${GITHUB_PAT:-}
if [ -z "$GITHUB_PAT" ]; then
  echo "Please set GITHUB_PAT environment variable (a PAT with repo scope)." >&2
  exit 1
fi

# derive owner/repo from git remote
ORIG_URL=$(git remote get-url origin || true)
if [[ "$ORIG_URL" =~ :([^/]+)/([^/]+)\.git$ ]]; then
  OWNER=${BASH_REMATCH[1]}
  REPO=${BASH_REMATCH[2]}
elif [[ "$ORIG_URL" =~ /([^/]+)/([^/]+)\.git$ ]]; then
  OWNER=${BASH_REMATCH[1]}
  REPO=${BASH_REMATCH[2]}
else
  echo "Could not determine OWNER/REPO from git remote; using defaults"
  OWNER="Yinhang3377"
  REPO="Rust-Blockchain-Secure-Wallet"
fi
API_BASE="https://api.github.com/repos/${OWNER}/${REPO}"

BRANCH=$(git rev-parse --abbrev-ref HEAD)
echo "Owner/Repo: ${OWNER}/${REPO}  Branch: ${BRANCH}"

# Download logs for a workflow run (requires PAT). Saves as .github/logs/run_<id>_<label>.zip
download_run_logs() {
  local run_id="$1"
  local label="${2:-ci}"
  mkdir -p .github/logs
  local out=".github/logs/run_${run_id}_${label}.zip"
  echo "Downloading logs for run ${run_id} -> ${out}"
  local code
  code=$(curl -sSL -o "$out" -w "%{http_code}" -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" \
    "${API_BASE}/actions/runs/${run_id}/logs") || true
  if [ "$code" != "200" ]; then
    echo "Failed to download logs for ${run_id} (HTTP $code). Ensure PAT has access or run is not public." >&2
    rm -f "$out" || true
    return 1
  fi
  echo "Logs saved to $out"
}

# create label if missing
create_label() {
  local name="$1"
  local color="$2"
  local desc="$3"
  encoded=$(python -c "import urllib.parse,sys;print(urllib.parse.quote(sys.argv[1]))" "$name")
  http_code=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: token ${GITHUB_PAT}" "$API_BASE/labels/${encoded}")
  if [ "$http_code" = "200" ]; then
    echo "label '${name}' exists"
  else
    echo "creating label '${name}'"
    curl -s -X POST -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" \
      "$API_BASE/labels" -d "{\"name\": \"${name}\", \"color\": \"${color}\", \"description\": \"${desc}\"}" | jq -r '.name'
  fi
}

create_label "autofix" "0e8a16" "Automated fixes created by CI autofix"
create_label "ci" "c2e0ff" "CI-related items"

echo "Labels ensured. Now pushing branch to origin..."
push_with_pat() {
  local branch="$1"
  # Validate token and resolve login to support PAT push
  local login
  login=$(curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" https://api.github.com/user | jq -r '.login // empty')
  if [ -z "$login" ]; then
    echo "Failed to validate PAT (no login returned). Ensure your PAT is valid and has 'repo' and 'workflow' scopes." >&2
  fi

  echo "Attempting push using GitHub App style token (x-access-token) ..."
  local url_app="https://x-access-token:${GITHUB_PAT}@github.com/${OWNER}/${REPO}.git"
  if git push "$url_app" "HEAD:${branch}"; then
    echo "Pushed via x-access-token style URL."
    return 0
  fi

  if [ -n "$login" ]; then
    echo "x-access-token push failed; attempting push using login:PAT ..."
    local url_pat="https://${login}:${GITHUB_PAT}@github.com/${OWNER}/${REPO}.git"
    if git push "$url_pat" "HEAD:${branch}"; then
      echo "Pushed via login:PAT URL."
      return 0
    fi
  fi

  echo "All push methods failed. Common causes: invalid PAT, insufficient scopes, or no permission to push to ${OWNER}/${REPO}." >&2
  echo "Ensure PAT scopes include: repo (all) and workflow. If pushing to a fork, confirm your PAT owner has write access." >&2
  return 1
}

push_with_pat "${BRANCH}"

echo "Pushed branch. Now polling Actions for CI runs on branch ${BRANCH}"

# helper to get latest run id for branch
get_latest_run_id_for_branch() {
  curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" \
    "${API_BASE}/actions/runs?branch=${BRANCH}&per_page=5" | jq -r '.workflow_runs[0].id // empty'
}

# poll for latest CI run and wait until it's completed
echo "Waiting for CI run to appear..."
CI_RUN_ID=""
for i in {1..60}; do
  CI_RUN_ID=$(get_latest_run_id_for_branch)
  if [ -n "$CI_RUN_ID" ]; then
    echo "Found run id: $CI_RUN_ID"
    break
  fi
  sleep 2
done
if [ -z "$CI_RUN_ID" ]; then
  echo "No CI run found for branch after waiting; aborting" >&2
  exit 2
fi

# wait for the run to complete
while true; do
  status_json=$(curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" "${API_BASE}/actions/runs/${CI_RUN_ID}")
  status=$(echo "$status_json" | jq -r .status)
  conclusion=$(echo "$status_json" | jq -r .conclusion)
  echo "CI run status=$status conclusion=$conclusion"
  if [ "$status" = "completed" ]; then
    break
  fi
  sleep 5
done

if [ "$conclusion" = "success" ]; then
  echo "CI run succeeded; no autofix expected."
  exit 0
fi

# Fetch logs for the failed CI run
download_run_logs "$CI_RUN_ID" "ci" || true

echo "CI run concluded with $conclusion (likely failure). Now polling for CI Auto-Fix workflow run."

# find workflow id for CI Auto-Fix
WORKFLOW_ID=$(curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" "${API_BASE}/actions/workflows" | jq -r '.workflows[] | select(.name=="CI Auto-Fix") | .id')
if [ -z "$WORKFLOW_ID" ]; then
  echo "Could not find workflow 'CI Auto-Fix' via API. Listing workflows for debugging:"
  curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" "${API_BASE}/actions/workflows" | jq -r '.workflows[].name + " -> " + (.id|tostring)'
  exit 1
fi

echo "CI Auto-Fix workflow id: $WORKFLOW_ID"

# poll for the ci-auto-fix run corresponding to the original run
AUTO_RUN_ID=""
for attempt in {1..60}; do
  AUTO_RUN_ID=$(curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" "${API_BASE}/actions/workflows/${WORKFLOW_ID}/runs?branch=${BRANCH}&per_page=5" | jq -r '.workflow_runs[0].id // empty')
  if [ -n "$AUTO_RUN_ID" ]; then
    echo "Found auto-fix run id: $AUTO_RUN_ID"
    break
  fi
  sleep 3
done

if [ -z "$AUTO_RUN_ID" ]; then
  echo "No CI Auto-Fix run found; maybe it didn't trigger. Check workflow triggers and permissions." >&2
  exit 1
fi

# wait for auto-fix run completion
while true; do
  status_json=$(curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" "${API_BASE}/actions/runs/${AUTO_RUN_ID}")
  status=$(echo "$status_json" | jq -r .status)
  conclusion=$(echo "$status_json" | jq -r .conclusion)
  echo "Auto-fix run status=$status conclusion=$conclusion"
  if [ "$status" = "completed" ]; then
    break
  fi
  sleep 5
done

# Fetch logs for the auto-fix run
download_run_logs "$AUTO_RUN_ID" "autofix" || true

# list open PRs and filter those with head.ref starting with autofix/
echo "Listing open PRs created recently with head refs starting with 'autofix/'"
curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" "${API_BASE}/pulls?state=open&sort=created&direction=desc" | jq -r '.[] | select(.head.ref|test("^autofix/")) | {number:.number,title:.title,head:.head.ref,html_url:.html_url}'

# list recent issues that might be autofix failures
echo "Listing recent issues that may indicate autofix failure (title contains 'autofix' or 'CI autofix failed')"
curl -s -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" "${API_BASE}/issues?state=open&since=$(date -I -d '1 day ago')" | jq -r '.[] | select(.title|test("autofix|CI autofix failed";"i")) | {number:.number,title:.title,html_url:.html_url}'

echo "Done. If a PR was created, you can review it on GitHub. If an issue was created after retries failed, it will also be listed above."

exit 0
