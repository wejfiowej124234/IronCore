#!/usr/bin/env bash
# NOTE: On Windows the executable bit may not be preserved; GitHub Actions will still run this with bash.
set -euo pipefail

# Enhanced auto-fix script:
# - apply cargo fmt and cargo fix
# - commit & push if changes
# - call Actions API to rerun the failed workflow
# - poll the rerun until completion; if still failing, retry up to MAX_ATTEMPTS

REPO_DIR=$(pwd)
BRANCH=$(git rev-parse --abbrev-ref HEAD)
OWNER_REPO=${GITHUB_REPOSITORY:-}
if [ -z "$OWNER_REPO" ]; then
  # derive from origin URL
  ORIG_URL=$(git remote get-url origin || true)
  # support git@github.com:owner/repo.git and https://github.com/owner/repo.git
  if [[ "$ORIG_URL" =~ :([^/]+)/([^/]+)\.git$ ]]; then
    OWNER=${BASH_REMATCH[1]}
    REPO=${BASH_REMATCH[2]}
    OWNER_REPO="${OWNER}/${REPO}"
  elif [[ "$ORIG_URL" =~ /([^/]+)/([^/]+)\.git$ ]]; then
    OWNER=${BASH_REMATCH[1]}
    REPO=${BASH_REMATCH[2]}
    OWNER_REPO="${OWNER}/${REPO}"
  else
    echo "Could not determine OWNER/REPO from GITHUB_REPOSITORY or git remote" >&2
    OWNER_REPO="unknown/unknown"
    OWNER="unknown"
    REPO="unknown"
  fi
else
  OWNER=${OWNER_REPO%%/*}
  REPO=${OWNER_REPO#*/}
fi
EVENT_FILE=${GITHUB_EVENT_PATH:-}

if [ -z "$EVENT_FILE" ]; then
  echo "GITHUB_EVENT_PATH not set; cannot determine workflow run context" >&2
  # continue but mark as no-api mode
  SKIP_API=1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq not installed; skipping GitHub API operations and performing local fixes only"
  SKIP_API=1
fi

if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "GITHUB_TOKEN not set or empty; skipping GitHub API operations and performing local fixes only"
  SKIP_API=1
fi

if [ -z "${SKIP_API:-}" ]; then
  ORIG_RUN_ID=$(jq -r .workflow_run.id < "$EVENT_FILE")
  WORKFLOW_ID=$(jq -r .workflow_run.workflow_id < "$EVENT_FILE")
  # head commit sha for the run (used for commit comments so it shows on the run page)
  HEAD_SHA=$(jq -r .workflow_run.head_sha < "$EVENT_FILE")
fi

MAX_ATTEMPTS=3
SLEEP_POLL=8
LAST_AUTOFIX_PR_URL=""
LAST_AUTOFIX_BRANCH=""

# Download logs zip for a workflow run and save under .github/logs
download_run_logs() {
  local run_id="$1"
  local label="${2:-ci}"
  mkdir -p .github/logs || true
  local out=".github/logs/run_${run_id}_${label}.zip"
  echo "Downloading logs for run ${run_id} -> ${out}"
  local code
  code=$(curl -sSL -o "$out" -w "%{http_code}" -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${OWNER}/${REPO}/actions/runs/${run_id}/logs") || true
  if [ "$code" != "200" ]; then
    echo "Failed to download logs for ${run_id} (HTTP $code)." >&2
    rm -f "$out" || true
    return 1
  fi
  echo "Logs saved to $out"
}

function do_local_fix() {
  echo "Running cargo fmt and cargo fix"
  cargo fmt --all
  cargo fix --all --allow-dirty --allow-staged || true
  if ! git diff --quiet; then
    echo "Found changes after autofix; committing"
    git config user.name "github-actions[bot]"
    git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
    git add -A
    git commit -m "chore(ci): auto-apply rustfmt/cargo-fix from CI autofix"
    if [ -z "${SKIP_API:-}" ]; then
      # Create a new branch for the autofix and push it
      NEW_BRANCH="autofix/${BRANCH}-${ORIG_RUN_ID}-${attempt}-$(date +%s)"
      git checkout -b "$NEW_BRANCH"
      git push "https://x-access-token:${GITHUB_TOKEN}@github.com/${OWNER}/${REPO}.git" "HEAD:${NEW_BRANCH}"
      echo "Pushed fixes to ${NEW_BRANCH}"
      LAST_AUTOFIX_BRANCH="$NEW_BRANCH"
      return 0
    else
      git push "https://x-access-token:${GITHUB_TOKEN}@github.com/${OWNER}/${REPO}.git" "HEAD:${BRANCH}" || true
      echo "Pushed fixes to ${BRANCH} (no-api mode)"
      return 0
    fi
  else
    echo "No local autofix changes"
    return 1
  fi
}

function create_pr_for_branch() {
  local new_branch="$1"
  local title="chore(ci): autofix suggestions (automated) for ${BRANCH} (run ${ORIG_RUN_ID})"
  local body="Automated fixes applied by CI autofix script.\n\nBranch: ${new_branch}\nOriginal run: https://github.com/${OWNER}/${REPO}/actions/runs/${ORIG_RUN_ID}\nAttempt: ${attempt}\n\nPlease review the changes in this PR."
  echo "Creating PR for branch ${new_branch} -> ${BRANCH}"
  resp=$(curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
    -d "{\"title\":\"${title}\",\"head\":\"${new_branch}\",\"base\":\"${BRANCH}\",\"body\":\"${body}\"}" \
    "https://api.github.com/repos/${OWNER}/${REPO}/pulls")
  PR_URL=$(echo "$resp" | jq -r .html_url)
  if [ "$PR_URL" = "null" ] || [ -z "$PR_URL" ]; then
    echo "Failed to create PR: $resp"
    return 1
  fi
  echo "PR created: ${PR_URL}"
  LAST_AUTOFIX_PR_URL="$PR_URL"
  # Add labels if provided
  if [ -n "${AUTOFIX_PR_LABELS:-}" ]; then
    echo "Adding labels: ${AUTOFIX_PR_LABELS} to PR"
    # labels should be comma-separated
    # Ensure labels exist (create if missing) then add them to the PR
    IFS=',' read -ra LABELS <<< "${AUTOFIX_PR_LABELS}"
    for lbl in "${LABELS[@]}"; do
      lbl_trimmed=$(echo "$lbl" | xargs)
      # check if label exists
      status_code=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: token ${GITHUB_TOKEN}" \
        "https://api.github.com/repos/${OWNER}/${REPO}/labels/$(python -c "import urllib.parse,sys;print(urllib.parse.quote(sys.argv[1]))" "$lbl_trimmed")")
      if [ "$status_code" = "404" ]; then
        echo "Label '$lbl_trimmed' not found; creating it"
        # create the label with a default color
        curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
          -d "{\"name\":\"${lbl_trimmed}\",\"color\":\"ededed\",\"description\":\"Autofix autogenerated label\"}" \
          "https://api.github.com/repos/${OWNER}/${REPO}/labels" >/dev/null || true
      fi
    done
    labels_json=$(jq -nc --arg l "${AUTOFIX_PR_LABELS}" '$l|split(",")')
    curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
      -d "{\"labels\":${labels_json}}" \
      "https://api.github.com/repos/${OWNER}/${REPO}/issues/$(echo "$resp" | jq -r .number)/labels"
  fi
  # Request reviewers if provided
  if [ -n "${AUTOFIX_REQUEST_REVIEWERS:-}" ]; then
    echo "Requesting reviewers: ${AUTOFIX_REQUEST_REVIEWERS}"
    reviewers_json=$(jq -nc --arg r "${AUTOFIX_REQUEST_REVIEWERS}" '{reviewers: ($r|split(",") ), team_reviewers: []}')
    curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
      -d "${reviewers_json}" \
      "https://api.github.com/repos/${OWNER}/${REPO}/pulls/$(echo "$resp" | jq -r .number)/requested_reviewers"
  fi
  # Assign assignees if provided
  if [ -n "${AUTOFIX_ASSIGNEES:-}" ]; then
    echo "Assigning users: ${AUTOFIX_ASSIGNEES} to PR"
    assignees_json=$(jq -nc --arg a "${AUTOFIX_ASSIGNEES}" '$a|split(",")')
    curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
      -d "{\"assignees\":${assignees_json}}" \
      "https://api.github.com/repos/${OWNER}/${REPO}/issues/$(echo \"$resp\" | jq -r .number)/assignees"
  fi
  # Post a commit comment to the run's head commit so the CI run page shows the PR link
  if [ -n "${HEAD_SHA:-}" ]; then
    PR_NUMBER=$(echo "$resp" | jq -r .number)
    commit_message="Automated autofix PR created: ${PR_URL}"
    curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
      -d "{\"body\":\"${commit_message}\"}" \
      "https://api.github.com/repos/${OWNER}/${REPO}/commits/${HEAD_SHA}/comments"
  fi
  # Create a Check Run so the PR link appears in the Checks tab for the workflow run
  if [ -n "${HEAD_SHA:-}" ]; then
    echo "Creating check run to surface PR link in Checks tab"
    check_req=$(jq -nc --arg name "Autofix PR" --arg head_sha "${HEAD_SHA}" --arg prurl "${PR_URL}" '{name:$name, head_sha:$head_sha, status:"completed", conclusion:"neutral", output:{title:"Autofix PR", summary:$prurl}}')
    curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
      -d "$check_req" "https://api.github.com/repos/${OWNER}/${REPO}/check-runs" >/dev/null || true
  fi
  return 0
}

function post_comment_on_run() {
  local run_id="$1"
  local message="$2"
  echo "Posting comment to workflow run ${run_id}"
  # GitHub doesn't provide direct run comments API; comments are typically placed on the PR or issue.
  # We'll post a comment on the PR and additionally create a short issue comment linking back to the run if needed.
  if [ -n "${LAST_AUTOFIX_PR_URL:-}" ]; then
    # Post comment on PR
    PR_NUMBER=$(echo "$LAST_AUTOFIX_PR_URL" | awk -F'/' '{print $NF}')
    curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
      -d "{\"body\":\"${message}\"}" \
      "https://api.github.com/repos/${OWNER}/${REPO}/issues/${PR_NUMBER}/comments"
  fi
  # Also create a lightweight issue for visibility referencing the run and PR (but mark it as triage-only)
  # This is optional and only done if requested via AUTOFIX_OPEN_ISSUE_ON_FAIL == "true" and we are in final failure handling.
}

function create_issue_for_failure() {
  local pr_url="${1:-}"
  local title="CI autofix failed after ${MAX_ATTEMPTS} attempts for run ${ORIG_RUN_ID}"
  local body="CI autofix attempted ${MAX_ATTEMPTS} times and the reruns did not succeed.\n\nBranch: ${BRANCH}\nOriginal run: https://github.com/${OWNER}/${REPO}/actions/runs/${ORIG_RUN_ID}\n"
  if [ -n "$pr_url" ]; then
    body+="A PR with the autofix changes was created: ${pr_url}\n\nPlease review the PR and apply manual fixes if necessary."
  else
    body+="No PR could be created automatically. Please investigate the failing run and apply fixes."
  fi
  echo "Creating issue to request human attention"
  resp=$(curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
    -d "{\"title\":\"${title}\",\"body\":\"${body}\"}" \
    "https://api.github.com/repos/${OWNER}/${REPO}/issues")
  ISSUE_URL=$(echo "$resp" | jq -r .html_url)
  echo "Issue created: ${ISSUE_URL}"
}

function rerun_workflow() {
  local run_id="$1"
  echo "Requesting rerun for workflow run ${run_id}"
  curl -s -X POST -H "Authorization: token ${GITHUB_TOKEN}" \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${OWNER}/${REPO}/actions/runs/${run_id}/rerun"
}

function poll_latest_run_for_workflow() {
  # Returns the latest run id for the workflow on this branch
  local workflow_id="$1"
  # Query the workflow runs for this workflow id and branch
  local runs_json
  runs_json=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${OWNER}/${REPO}/actions/workflows/${workflow_id}/runs?branch=${BRANCH}&per_page=5")
  echo "$runs_json" | jq -r '.workflow_runs[] | select(.head_branch==env.BRANCH) | .id' | head -n1
}

function get_run_status() {
  local run_id="$1"
  curl -s -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${OWNER}/${REPO}/actions/runs/${run_id}" | jq -r '.status, .conclusion'
}

if [ -z "${SKIP_API:-}" ]; then
  echo "Autofix: origin run id=${ORIG_RUN_ID}, workflow_id=${WORKFLOW_ID}, branch=${BRANCH}"
  # Attempt to capture logs of the original failed run for diagnostics
  download_run_logs "${ORIG_RUN_ID}" "failed" || true
else
  echo "Autofix: running in local-only mode; branch=${BRANCH}"
fi

attempt=1
while [ $attempt -le $MAX_ATTEMPTS ]; do
  echo "Autofix attempt ${attempt}/${MAX_ATTEMPTS}"

  if [ -n "${SKIP_API:-}" ]; then
    # Only perform local fixes and exit accordingly
    if do_local_fix; then
      echo "Local fixes applied (no API); exiting"
      exit 0
    else
      echo "No local fixes applied and API unavailable; exiting with code 1"
      exit 1
    fi
  fi

  do_local_fix || echo "No local fixes to push"

  # Trigger a rerun of the original failed run
  rerun_workflow "${ORIG_RUN_ID}"

  # Wait a short moment for the new run to be created
  sleep 3

  # Find the newest run for this workflow on this branch
  NEW_RUN_ID=$(poll_latest_run_for_workflow "${WORKFLOW_ID}")
  if [ -z "$NEW_RUN_ID" ] || [ "$NEW_RUN_ID" = "null" ]; then
    echo "Could not find new run id for workflow ${WORKFLOW_ID} on branch ${BRANCH}"
    exit 1
  fi
  echo "Monitoring rerun id: ${NEW_RUN_ID}"

  # Poll until completed
  while true; do
    read -r status conclusion <<<"$(get_run_status "$NEW_RUN_ID")"
    echo "Run ${NEW_RUN_ID} status=${status} conclusion=${conclusion}"
    if [ "$status" = "completed" ]; then
      break
    fi
    sleep $SLEEP_POLL
  done

  if [ "$conclusion" = "success" ]; then
    echo "Rerun ${NEW_RUN_ID} succeeded. Exiting with success."
    # Save rerun logs as well
    download_run_logs "${NEW_RUN_ID}" "rerun" || true
    # If we created a PR earlier in this attempt, post a comment to link back to the run (for traceability)
    if [ -n "${LAST_AUTOFIX_PR_URL:-}" ]; then
      post_comment_on_run "$NEW_RUN_ID" "Automated autofix PR created: ${LAST_AUTOFIX_PR_URL}. Rerun succeeded."
    fi
    exit 0
  else
    echo "Rerun ${NEW_RUN_ID} concluded with '${conclusion}'"
    # Save rerun logs for debugging
    download_run_logs "${NEW_RUN_ID}" "rerun" || true
    # If we pushed a branch, attempt to create a PR for review (no auto-merge)
    if [ -n "${LAST_AUTOFIX_BRANCH:-}" ]; then
      create_pr_for_branch "${LAST_AUTOFIX_BRANCH}" || echo "PR creation failed"
      # Post link to PR on the run for traceability
      if [ -n "${LAST_AUTOFIX_PR_URL:-}" ]; then
        post_comment_on_run "$NEW_RUN_ID" "Automated autofix PR created: ${LAST_AUTOFIX_PR_URL}. Please review."
      fi
    fi
    attempt=$((attempt + 1))
    # Try to apply fixes again in next loop iteration
  fi
done

echo "All ${MAX_ATTEMPTS} attempts exhausted; creating issue for human review if possible"
if [ -z "${SKIP_API:-}" ]; then
  create_issue_for_failure "${LAST_AUTOFIX_PR_URL:-}"
fi
exit 2
