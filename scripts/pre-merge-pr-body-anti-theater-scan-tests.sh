#!/usr/bin/env bash
# pre-merge-pr-body-anti-theater-scan-tests.sh — hermetic smoke test
# harness for `pre-merge-pr-body-anti-theater-scan.sh` (gate 9).
#
# No real `gh` calls, no real SHARED.md writes — both
# `FAKE_PR_BODY` and `FAKE_SHARED_MD_PATH` redirect the helper script
# to in-memory / tempfile inputs. Pattern precedent: gateway-freshness
# harness uses `FAKE_HEALTH_BODY` / `FAKE_HEALTH_STATUS` the same way.
#
# Usage:
#   bash scripts/pre-merge-pr-body-anti-theater-scan-tests.sh
#
# Exit codes:
#   0 — all scenarios passed
#   1 — at least one scenario failed

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

SCAN_SCRIPT="$REPO_ROOT/scripts/pre-merge-pr-body-anti-theater-scan.sh"
if [[ ! -x "$SCAN_SCRIPT" ]]; then
  printf '\033[31mFAIL\033[0m scan script not executable at %s\n' "$SCAN_SCRIPT" >&2
  exit 1
fi

pass_count=0
fail_count=0
TMPDIR_HARNESS="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_HARNESS"' EXIT

run_case() {
  local name="$1"
  local expected_rc="$2"
  shift 2
  local actual_rc=0
  local stdout
  if stdout="$("$@" 2>&1)"; then
    actual_rc=0
  else
    actual_rc=$?
  fi
  if [[ "$actual_rc" -eq "$expected_rc" ]]; then
    printf '\033[32mPASS\033[0m %s (exit %d)\n' "$name" "$actual_rc"
    pass_count=$((pass_count + 1))
  else
    printf '\033[31mFAIL\033[0m %s (expected exit %d, got %d)\n' "$name" "$expected_rc" "$actual_rc" >&2
    printf -- '---output---\n%s\n------------\n' "$stdout" >&2
    fail_count=$((fail_count + 1))
  fi
}

# Scenario 1: GREEN — body with no admission phrase, exit 0.
CLEAN_BODY="$(cat <<'EOF'
## Summary
Adds a new column to the audit_log table.

## Test plan
- cargo test -p argo-store passed cold (5 tests, 0 failures).
- shellcheck on scripts/check-audit-log-column.sh clean.

🤖 Generated with Claude Code
EOF
)"
FAKE_PR_BODY="$CLEAN_BODY" \
  run_case "scenario 1: clean body exits 0" 0 \
  bash "$SCAN_SCRIPT" --pr 9999 --quiet

# Scenario 2: RED — admission phrase present, no override, exit 1.
DIRTY_BODY="$(cat <<'EOF'
## Summary
Fixes the seeding race.

## Test plan
- Local cargo check was interrupted by file-lock contention from
  concurrent worker compilation; the formal pre-merge gate will run
  before admin-merge.

🤖 Generated with Claude Code
EOF
)"
FAKE_PR_BODY="$DIRTY_BODY" \
  run_case "scenario 2: admission phrase, no override, exits 1" 1 \
  bash "$SCAN_SCRIPT" --pr 2437 --quiet

# Scenario 3: GREEN — admission phrase + --allow-anti-theater-admit,
# exit 0 + SHARED.md trail appended to FAKE path.
SHARED_PATH="$TMPDIR_HARNESS/SHARED.md"
mkdir -p "$(dirname "$SHARED_PATH")"
: > "$SHARED_PATH"
FAKE_PR_BODY="$DIRTY_BODY" FAKE_SHARED_MD_PATH="$SHARED_PATH" \
  run_case "scenario 3: admission + override, exits 0" 0 \
  bash "$SCAN_SCRIPT" --pr 2437 --allow-anti-theater-admit --quiet
# Verify the SHARED.md trail line was appended with the expected
# anchor marker.
if grep -q '\[anti-theater-admit\] PR #2437' "$SHARED_PATH"; then
  printf '\033[32mPASS\033[0m   scenario 3 sub-check: SHARED.md trail line present\n'
  pass_count=$((pass_count + 1))
else
  printf '\033[31mFAIL\033[0m   scenario 3 sub-check: SHARED.md trail line missing\n' >&2
  printf -- '---SHARED.md contents---\n' >&2
  cat "$SHARED_PATH" >&2
  printf -- '------------------------\n' >&2
  fail_count=$((fail_count + 1))
fi

# Scenario 4: GREEN — no --pr given, scan skips cleanly, exit 0.
run_case "scenario 4: no --pr, skip, exits 0" 0 \
  bash "$SCAN_SCRIPT" --quiet

printf '\n'
if [[ "$fail_count" -gt 0 ]]; then
  printf '\033[31mFAIL\033[0m %d scenario(s) failed (%d passed)\n' "$fail_count" "$pass_count" >&2
  exit 1
fi
printf '\033[32mPASS\033[0m %d scenarios green\n' "$pass_count"
exit 0
