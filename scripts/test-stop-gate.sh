#!/usr/bin/env bash
# test-stop-gate.sh — the DEC-042 Stage-1 B-2 META-VERIFIER (the deliverable).
#
# It proves the property the brief demands: "a worker that leaves a failing test
# cannot exit 'done'; the hook blocks it." Self-contained, deterministic, NO real
# compile and NO network — gate outcomes are injected via the documented env
# seams (STOP_GATE_VERIFY_CMD / STOP_GATE_*_CMD). Runs the whole suite twice and
# asserts identical results.
#
# Cases:
#   A. ARGO_STOP_GATE=1 + FAILING gate  -> row held at NEEDS-FIX, reason recorded,
#      enforce returns non-zero (a failing-gate change can NOT reach IN_REVIEW).
#   B. ARGO_STOP_GATE=1 + PASSING gate  -> row stays IN_REVIEW, enforce returns 0.
#   C. ARGO_STOP_GATE unset (no-behavior-change) -> row stays IN_REVIEW, verifier
#      is NOT even invoked, enforce returns 0.
#   D. verifier composition: ARGO_STOP_GATE=1 + a failing injected gate command
#      -> stop-gate-verify.sh exits 1 with a STOP_GATE_FAIL line.
#   E. verifier disabled no-op: ARGO_STOP_GATE unset -> STOP_GATE_PASS, exit 0.

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENFORCE_SRC="$ROOT/scripts/stop-gate-enforce.sh"
VERIFY="$ROOT/scripts/stop-gate-verify.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/stop-gate-test.XXXXXX")"
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
assert_stdout_has() {  # text pattern msg
  if printf '%s\n' "$1" | grep -q "$2"; then ok "$3"; else bad "$3 (pattern '$2' absent)"; fi
}

# Build a throwaway ledger with one IN_REVIEW row for a fake brief.
make_repo() {
  local d="$1"
  rm -rf "$d"; mkdir -p "$d/docs/build-plan"
  cat > "$d/docs/build-plan/BUILD_STATUS.md" <<'EOF'
| brief-id | owner | status | pr | notes |
|---|---|---|---|---|
| stop-gate-fake-brief | claude | IN_REVIEW | #9001 | shipped via worker; opened PR #9001 |
EOF
}

# Extract the status column (field 4) of the fake brief's row.
row_status() {
  grep -E "^\| stop-gate-fake-brief \|" "$1/docs/build-plan/BUILD_STATUS.md" \
    | head -1 | awk -F'|' '{gsub(/^[ \t]+|[ \t]+$/,"",$4); print $4}'
}

run_enforce() {  # $1=flag  $2=verify_cmd  -> echoes "rc:<rc>"
  local d="$WORK/repo"
  make_repo "$d"
  ( set +e
    # shellcheck source=scripts/stop-gate-enforce.sh disable=SC1091
    . "$ENFORCE_SRC"
    ARGO_STOP_GATE="$1" STOP_GATE_VERIFY_CMD="$2" STOP_GATE_SKIP_PR_COMMENT=1 \
      stop_gate_enforce "stop-gate-fake-brief" "claude-stop-gate-fake-brief" "$d"
    echo "rc:$?"
  )
}

suite() {
  local round="$1"
  echo "── round $round ─────────────────────────────────────────"

  local ledger="$WORK/repo/docs/build-plan/BUILD_STATUS.md"
  local out rc st marker

  # Case A: failing gate, flag on -> NEEDS-FIX + non-zero + reason note.
  out="$(run_enforce 1 'echo "STOP_GATE_FAIL: injected failing cargo test"; exit 1')"
  rc="$(printf '%s\n' "$out" | sed -n 's/^rc://p' | tail -1)"
  st="$(row_status "$WORK/repo")"
  assert_eq "$st" "NEEDS-FIX" "A: failing gate held row at NEEDS-FIX (not IN_REVIEW)"
  assert_ne "$rc" "0"         "A: enforce returned non-zero"
  assert_grep "HELD by stop-gate"        "$ledger" "A: hold reason recorded in the row"
  assert_grep "injected failing cargo test" "$ledger" "A: specific gate reason carried through"

  # Case B: passing gate, flag on -> stays IN_REVIEW + zero.
  out="$(run_enforce 1 'echo STOP_GATE_PASS; exit 0')"
  rc="$(printf '%s\n' "$out" | sed -n 's/^rc://p' | tail -1)"
  st="$(row_status "$WORK/repo")"
  assert_eq "$st" "IN_REVIEW" "B: passing gate left row at IN_REVIEW"
  assert_eq "$rc" "0"         "B: enforce returned 0"

  # Case C: flag UNSET -> no-behavior-change. Verifier must NOT be invoked.
  marker="$WORK/verify-was-called-$round"
  rm -f "$marker"
  out="$(run_enforce 0 "touch '$marker'; echo 'STOP_GATE_FAIL: should never run'; exit 1")"
  rc="$(printf '%s\n' "$out" | sed -n 's/^rc://p' | tail -1)"
  st="$(row_status "$WORK/repo")"
  assert_eq "$st" "IN_REVIEW" "C: flag-off left row byte-identical (IN_REVIEW)"
  assert_eq "$rc" "0"         "C: flag-off enforce returned 0"
  assert_absent "$marker"     "C: verifier never invoked with flag off"

  # Case D: verifier composition — injected failing gate surfaces STOP_GATE_FAIL.
  out="$(ARGO_STOP_GATE=1 STOP_GATE_LOC_CMD='exit 0' STOP_GATE_LEDGER_CMD='exit 0' \
         STOP_GATE_TEST_CMD='exit 1' STOP_GATE_CLIPPY_CMD='exit 0' \
         bash "$VERIFY" --brief x --branch y 2>/dev/null)"
  rc=$?
  assert_eq "$rc" "1"                       "D: verifier exit 1 on failing gate"
  assert_stdout_has "$out" '^STOP_GATE_FAIL:' "D: verifier printed STOP_GATE_FAIL reason"

  # Case E: verifier disabled no-op.
  out="$(ARGO_STOP_GATE=0 bash "$VERIFY" --brief x --branch y 2>/dev/null)"; rc=$?
  assert_eq "$rc" "0"                        "E: disabled verifier exits 0"
  assert_stdout_has "$out" '^STOP_GATE_PASS'  "E: disabled verifier is a PASS no-op"
}

suite 1
suite 2

echo "─────────────────────────────────────────────────────────"
echo "stop-gate meta-verifier: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
echo "ALL GREEN — done binds to gate-verified."
