#!/usr/bin/env bash
set -euo pipefail


ROOT_DIR=$(dirname "$(dirname "$(realpath "$0")")")
echo "Running forbidden-print scan in $ROOT_DIR"

# Prefer ripgrep for speed and IDE/ignore awareness; fall back to grep if not available.
if command -v rg >/dev/null 2>&1; then
  matches=$(rg --hidden --no-ignore-vcs --line-number "(println!|eprintln!|dbg!)" src --glob '!src/bin/**' || true)
else
  echo "rg: not found, falling back to grep" >&2
  # grep fallback: search recursively, exclude src/bin and tests directories
  matches=$(grep -ERn --exclude-dir=src/bin --exclude-dir=tests --exclude-dir=.git -E "(println!|eprintln!|dbg!)" src || true)
fi

if [[ -n "$matches" ]]; then
  # Filter out matches that are inside test files or src/bin
  violations=""
  while IFS= read -r line; do
    # Each line is: path:line:content
    file=$(echo "$line" | cut -d: -f1)

    # Skip any files under src/bin
    if [[ "$file" == src/bin/* ]]; then
      continue
    fi

    # If file name ends with _tests.rs, treat as test-only and skip
    if [[ "$file" =~ _tests\.rs$ ]]; then
      continue
    fi

    # If file content contains #[cfg(test)] or mod tests, it's test-only (skip)
    if grep -Eq "#\s*\[cfg\s*\(\s*test" "$file" || grep -Eq "mod\s+tests" "$file"; then
      continue
    fi

    violations+="$line\n"
  done <<< "$matches"

  if [[ -n "$violations" ]]; then
    echo "Forbidden print macros found in non-test sources:" >&2
    echo -e "$violations" >&2
    exit 1
  fi
fi

echo "No forbidden print macros found (excluding tests and src/bin)."
