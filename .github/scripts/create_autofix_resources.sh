#!/usr/bin/env bash
# Create repo labels and print useful GitHub API commands for PRs/issues/check-runs
# Usage:
#   export GITHUB_PAT="ghp_..."
#   bash .github/scripts/create_autofix_resources.sh

set -euo pipefail

OWNER="Yinhang3377"
REPO="Rust-Blockchain-Secure-Wallet"

if [ -z "${GITHUB_PAT:-}" ]; then
  echo "Please set GITHUB_PAT environment variable (a PAT with repo scope)."
  exit 1
fi

API_BASE="https://api.github.com/repos/${OWNER}/${REPO}"

create_label() {
  local name="$1"
  local color="$2"
  local desc="$3"

  echo "Checking label: $name"
  status=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: token ${GITHUB_PAT}" "$API_BASE/labels/$(python -c "import urllib.parse,sys;print(urllib.parse.quote(sys.argv[1]))" "$name")")
  if [ "$status" = "200" ]; then
    echo "  Label '$name' already exists"
  else
    echo "  Creating label '$name'"
    curl -s -X POST -H "Authorization: token ${GITHUB_PAT}" -H "Accept: application/vnd.github+json" \
      "$API_BASE/labels" -d "{\"name\": \"${name}\", \"color\": \"${color}\", \"description\": \"${desc}\"}" | jq -r '.name'
  fi
}

# Create labels
create_label "autofix" "0e8a16" "Automated fixes created by CI autofix"
create_label "ci" "c2e0ff" "CI-related items"

cat <<'EOF'

Labels ensured.

Useful manual commands (replace placeholders as needed).

# Create a PR (requires a branch 'autofix/test-branch' pushed to remote):
# curl -X POST -H "Authorization: token <PAT>" -H "Accept: application/vnd.github+json" \
#   https://api.github.com/repos/<OWNER>/<REPO>/pulls \
#   -d '{"title":"chore(ci): test autofix PR","head":"autofix/test-branch","base":"main","body":"Test PR."}'

# Add labels to PR/issue (PRs are issues in API):
# curl -X POST -H "Authorization: token <PAT>" -H "Accept: application/vnd.github+json" \
#   https://api.github.com/repos/<OWNER>/<REPO>/issues/<PR_NUMBER>/labels \
#   -d '{"labels":["autofix","ci"]}'

# Request reviewers for a PR:
# curl -X POST -H "Authorization: token <PAT>" -H "Accept: application/vnd.github+json" \
#   https://api.github.com/repos/<OWNER>/<REPO>/pulls/<PR_NUMBER>/requested_reviewers \
#   -d '{"reviewers":["alice","bob"],"team_reviewers":["org/team-slug"]}'

# Assign assignees to PR/issue:
# curl -X POST -H "Authorization: token <PAT>" -H "Accept: application/vnd.github+json" \
#   https://api.github.com/repos/<OWNER>/<REPO>/issues/<PR_NUMBER>/assignees \
#   -d '{"assignees":["alice"]}'

# Create commit comment (shows on workflow run page):
# curl -X POST -H "Authorization: token <PAT>" -H "Accept: application/vnd.github+json" \
#   https://api.github.com/repos/<OWNER>/<REPO>/commits/<HEAD_SHA>/comments \
#   -d '{"body":"Autofix PR created: <PR_URL>"}'

# Create a Check Run (shows in Checks tab):
# curl -X POST -H "Authorization: token <PAT>" -H "Accept: application/vnd.github+json" \
#   https://api.github.com/repos/<OWNER>/<REPO>/check-runs \
#   -d '{"name":"Autofix PR","head_sha":"<HEAD_SHA>","status":"completed","conclusion":"neutral","output":{"title":"Autofix PR","summary":"Automated autofix PR: <PR_URL>"}}'

EOF

echo "Done. Run the commented commands (with your PAT) to create PRs / request reviewers / assign, etc." > /dev/stderr

exit 0
