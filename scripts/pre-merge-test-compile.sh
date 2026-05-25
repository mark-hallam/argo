#!/usr/bin/env bash
# pre-merge-test-compile.sh — verify all test crates compile against the
# current workspace. Closes the gap where `--skip-clippy` is the only
# cross-crate sanity check and `cargo test` can land red on origin/main
# because test-side compile errors slip past per-crate worker checks.
#
# Invoked by `scripts/conductor-pre-merge-gate.sh` as gate 6 and by the
# smoke harness `scripts/conductor-pre-merge-gate-tests.sh`.
#
# Allowlist: `scripts/pre-merge-test-carry-allowlist.txt` — when every
# failing source file matches at least one substring listed there, the
# gate prints PASS-WITH-WARNING and exits 0. Carries are debt, not a
# workaround; the conductor periodically prunes stale entries.
#
# Usage:
#   bash scripts/pre-merge-test-compile.sh
#   bash scripts/pre-merge-test-compile.sh --quiet     # only print on failure / warning
#
# Exit codes:
#   0 — all test crates compile (or all failures are allowlisted)
#   1 — at least one non-allowlisted compile error

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

QUIET=0
[[ "${1:-}" == "--quiet" ]] && QUIET=1

ALLOWLIST_FILE="scripts/pre-merge-test-carry-allowlist.txt"

# Read allowlist patterns (strip comments / blank lines)
patterns=()
if [[ -f "$ALLOWLIST_FILE" ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    [[ -z "$trimmed" || "$trimmed" == \#* ]] && continue
    patterns+=("$trimmed")
  done < "$ALLOWLIST_FILE"
fi

start=$SECONDS
[[ "$QUIET" -eq 0 ]] && echo "=== test-compile (cargo check --workspace --tests) ==="

# --tests scopes the build to test targets only (excludes binaries,
# examples, benches). Faster than --all-targets and exactly what we
# want to gate on: "do the tests compile against current workspace?"
output=$(cargo check --workspace --tests 2>&1)
rc=$?
elapsed=$((SECONDS - start))

if [[ "$rc" -eq 0 ]]; then
  [[ "$QUIET" -eq 0 ]] && echo "✅ all test crates compile (${elapsed}s)"
  exit 0
fi

# cargo check failed — collect error-bearing source paths.
# cargo's error block format anchors each error on a "--> path:LINE:COL" line.
error_files=$(echo "$output" \
  | grep -E '^[[:space:]]*--> ' \
  | sed -E 's/^[[:space:]]*--> //; s/:[0-9]+:[0-9]+$//' \
  | sort -u)

if [[ -z "$error_files" ]]; then
  echo "❌ test-compile FAILED (${elapsed}s) — cargo exit $rc without source-pointed errors" >&2
  echo "$output" | tail -40 >&2
  exit 1
fi

allowlisted_files=()
unallowlisted_files=()
while IFS= read -r f; do
  matched=0
  for pat in "${patterns[@]}"; do
    if [[ "$f" == *"$pat"* ]]; then
      matched=1
      break
    fi
  done
  if [[ "$matched" -eq 1 ]]; then
    allowlisted_files+=("$f")
  else
    unallowlisted_files+=("$f")
  fi
done <<< "$error_files"

if [[ "${#unallowlisted_files[@]}" -eq 0 ]]; then
  echo "⚠️  test-compile PASS-WITH-WARNING (${elapsed}s)"
  echo ""
  echo "All compile errors are in allowlisted files (carried per"
  echo "$ALLOWLIST_FILE):"
  for f in "${allowlisted_files[@]}"; do
    echo "   - $f"
  done
  echo ""
  echo "Carries are debt — clear them or remove the entry."
  exit 0
fi

echo "❌ test-compile FAILED (${elapsed}s)"
echo ""
echo "Compile errors in non-allowlisted files:"
for f in "${unallowlisted_files[@]}"; do
  echo "   - $f"
done
if [[ "${#allowlisted_files[@]}" -gt 0 ]]; then
  echo ""
  echo "Also failing (allowlisted; would pass alone):"
  for f in "${allowlisted_files[@]}"; do
    echo "   - $f"
  done
fi
echo ""
echo "Last 60 lines of cargo output:"
echo "$output" | tail -60
echo ""
echo "Re-run unfiltered: cargo check --workspace --tests"
echo "Carry a known red: add a path-substring to $ALLOWLIST_FILE"
exit 1
