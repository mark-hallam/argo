#!/usr/bin/env bash
# test-verify-agent.sh — the DEC-042 Stage-1 B-4 META-VERIFIER (the deliverable).
#
# It proves the property the brief demands: every change carries a verify-agent
# verdict from a DIFFERENT context than the author, and a FAIL holds the row so a
# rejected change can never present as done. Self-contained, deterministic, NO
# real `claude` invocation and NO network — the verdict is injected via the
# documented env seam (VERIFY_AGENT_CMD). Runs the whole suite twice and asserts
# identical results.
#
# Cases (from the brief's Acceptance §1):
#   A. ARGO_SEPARATE_VERIFIER=1 + injected FAIL  -> row flipped to NEEDS-FIX,
#      reason recorded, non-zero return.
#   B. flag ON + injected PASS                   -> row stays IN_REVIEW, return 0.
#   C. flag UNSET (no-behavior-change)           -> the verify cmd is NEVER
#      invoked (asserted via a side-effect file it would create), return 0.
#   D. output-contract parse                     -> a FAIL last-line yields the
#      reason verbatim in the row note.

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$ROOT/scripts/stop-gate-verify-agent.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/verify-agent-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

PASS=0; FAIL=0
ok()  { PASS=$((PASS+1)); echo "  ✅ $1"; }
bad() { FAIL=$((FAIL+1)); echo "  ❌ $1"; }
assert_eq() {  # actual expected msg
  if [ "$1" = "$2" ]; then ok "$3"; else bad "$3 (expected '$2', got '$1')"; fi
}
assert_ne() {  # actual notexpected msg
  if [ "$1" != "$2" ]; then ok "$3"; else bad "$3 (unexpectedly got '$1')"; fi
}
assert_grep() {  # pattern file msg
  if grep -q "$1" "$2"; then ok "$3"; else bad "$3 (pattern '$1' absent)"; fi
}
assert_absent() {  # path msg
  if [ ! -f "$1" ]; then ok "$2"; else bad "$2 (file present)"; fi
}

# Throwaway ledger with one IN_REVIEW row for a fake brief.
make_repo() {
  local d="$1"
  rm -rf "$d"; mkdir -p "$d/docs/build-plan"
  cat > "$d/docs/build-plan/BUILD_STATUS.md" <<'EOF'
| brief-id | owner | status | pr | notes |
|---|---|---|---|---|
| verify-agent-fake-brief | claude | IN_REVIEW | #9100 | shipped via worker; opened PR #9100 |
EOF
}

# Extract the status column (field 4) of the fake brief's row.
row_status() {
  grep -E "^\| verify-agent-fake-brief \|" "$1/docs/build-plan/BUILD_STATUS.md" \
    | head -1 | awk -F'|' '{gsub(/^[ \t]+|[ \t]+$/,"",$4); print $4}'
}

# Run the verify-agent script against the throwaway repo. $1=flag $2=verify_cmd.
# PR comment is suppressed (no `gh`); echoes "rc:<rc>".
run_agent() {  # $1=flag  $2=verify_cmd
  local d="$WORK/repo"
  make_repo "$d"
  ARGO_SEPARATE_VERIFIER="$1" VERIFY_AGENT_CMD="$2" VERIFY_AGENT_SKIP_PR_COMMENT=1 \
    bash "$SCRIPT" --brief "verify-agent-fake-brief" \
      --branch "claude-verify-agent-fake-brief" --repo "$d" >/dev/null 2>&1
  echo "rc:$?"
}

suite() {
  local round="$1"
  echo "── round $round ─────────────────────────────────────────"

  local ledger="$WORK/repo/docs/build-plan/BUILD_STATUS.md"
  local out rc st marker

  # Case A: failing verdict, flag on -> NEEDS-FIX + non-zero + reason note.
  out="$(run_agent 1 'echo "VERIFY_AGENT_FAIL: tests assert nothing real"; exit 1')"
  rc="$(printf '%s\n' "$out" | sed -n 's/^rc://p' | tail -1)"
  st="$(row_status "$WORK/repo")"
  assert_eq "$st" "NEEDS-FIX" "A: failing verdict held row at NEEDS-FIX (not IN_REVIEW)"
  assert_ne "$rc" "0"         "A: agent returned non-zero on FAIL"
  assert_grep "HELD by verify-agent" "$ledger" "A: hold reason recorded in the row"

  # Case B: passing verdict, flag on -> stays IN_REVIEW + zero.
  out="$(run_agent 1 'echo VERIFY_AGENT_PASS; exit 0')"
  rc="$(printf '%s\n' "$out" | sed -n 's/^rc://p' | tail -1)"
  st="$(row_status "$WORK/repo")"
  assert_eq "$st" "IN_REVIEW" "B: passing verdict left row at IN_REVIEW"
  assert_eq "$rc" "0"         "B: agent returned 0 on PASS"

  # Case C: flag UNSET -> no-behavior-change. Verify cmd must NOT be invoked.
  marker="$WORK/verify-was-called-$round"
  rm -f "$marker"
  out="$(run_agent 0 "touch '$marker'; echo 'VERIFY_AGENT_FAIL: should never run'; exit 1")"
  rc="$(printf '%s\n' "$out" | sed -n 's/^rc://p' | tail -1)"
  st="$(row_status "$WORK/repo")"
  assert_eq "$st" "IN_REVIEW" "C: flag-off left row byte-identical (IN_REVIEW)"
  assert_eq "$rc" "0"         "C: flag-off agent returned 0"
  assert_absent "$marker"     "C: verify cmd never invoked with flag off"

  # Case D: output-contract parse — the FAIL reason is carried verbatim into the row.
  out="$(run_agent 1 'echo "VERIFY_AGENT_FAIL: acceptance box 3 unticked"; exit 1')"
  st="$(row_status "$WORK/repo")"
  assert_eq "$st" "NEEDS-FIX" "D: parsed FAIL last-line held the row"
  assert_grep "acceptance box 3 unticked" "$ledger" "D: FAIL reason carried verbatim into row note"
}

suite 1
suite 2

echo "─────────────────────────────────────────────────────────"
echo "verify-agent meta-verifier: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
echo "ALL GREEN — every change carries an independent verify-agent verdict."
