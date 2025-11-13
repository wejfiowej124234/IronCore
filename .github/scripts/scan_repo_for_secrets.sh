#!/usr/bin/env bash
set -euo pipefail

# Scan repo for risky patterns (plaintext prints and Debug-format usage) in non-test files.
# Whitelist is used for allowed files that intentionally print small info for CLI.

WHITELIST=(
  "src/bin/wallet-cli.rs"
  "src/bin/nonce_harness.rs"
  "src/crypto/hsm.rs"
)

echo "Scanning source for risky patterns..."

# Patterns to search for
# Match println!/eprintln!/format!("{:?}") and other debug-format variants
PATTERN='(println!\(|eprintln!\(|format!\(\"\{:\?\}\"|format!\(\"\\\{:\\?\}\\\" )'

# Gather matches (ignore target directory)
matches=$(grep -nR --binary-files=without-match -E "$PATTERN" src || true)

bad_matches=""
if [ -n "$matches" ]; then
  while IFS= read -r line; do
    file=$(echo "$line" | cut -d: -f1)
    # skip test files and test directories
    if [[ "$file" =~ _tests.rs$ ]] || [[ "$file" =~ ^tests/ ]] || [[ "$file" =~ /tests/ ]]; then
      continue
    fi
    # skip bin/tooling directories (command-line helpers) and generator tooling
    if [[ "$file" =~ ^src/bin/ ]] || [[ "$file" =~ ^src/tools/ ]]; then
      continue
    fi
    # skip whitelisted files
    skip=false
    for w in "${WHITELIST[@]}"; do
      if [[ "$file" == "$w" ]]; then
        skip=true
        break
      fi
    done
    if ! $skip; then
      bad_matches+="$line\n"
    fi
  done <<< "$matches"
fi

if [ -n "$bad_matches" ]; then
  echo "Found risky print/Debug patterns in non-test source files:" >&2
  echo -e "$bad_matches" >&2
  exit 1
fi

echo "No un-whitelisted plaintext prints or Debug-format patterns found in non-test source files."
exit 0
