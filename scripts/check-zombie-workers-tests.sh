#!/usr/bin/env bash
# check-zombie-workers-tests.sh — smoke tests for check-zombie-workers.sh.
#
# Coverage:
#   1. Empty pool (no claude workers) prints "(no claude workers alive)"
#      and exits 0. Skipped if claude workers are actually running on
#      this machine (would false-fail).
#   2. Synthetic fixture exercising all three tiers (🟢/🟡/🔴) — uses
#      --fixture mode so it's deterministic, no live processes needed.
#
# Usage:
#   bash scripts/check-zombie-workers-tests.sh
#
# Exit codes:
#   0 — all tests passed (or skipped where unsafe)
#   1 — one or more tests failed

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SUT="$SCRIPT_DIR/check-zombie-workers.sh"

if [[ ! -x "$SUT" ]]; then
  echo "FAIL: $SUT is not executable" >&2
  exit 1
fi

PASS=0
FAIL=0

pass() { echo "PASS: $*"; PASS=$((PASS + 1)); }
fail() { echo "FAIL: $*" >&2; FAIL=$((FAIL + 1)); }

# ------------------------------------------------------------------
# Test 1: empty pool
# ------------------------------------------------------------------
if pgrep -f "claude --dangerously-skip-permissions" >/dev/null 2>&1; then
  echo "SKIP: live claude workers detected; empty-pool test would false-fail."
else
  output=$(bash "$SUT" 2>&1)
  rc=$?
  if [[ "$rc" -eq 0 ]] && [[ "$output" == *"(no claude workers alive)"* ]]; then
    pass "empty pool returns 0 + '(no claude workers alive)'"
  else
    fail "empty pool — rc=$rc, output=$output"
  fi
fi

# ------------------------------------------------------------------
# Test 2: synthetic fixture covering all three tiers
# ------------------------------------------------------------------
FIXTURE=$(mktemp -t zombie-fixture.XXXXXX)
trap 'rm -f "$FIXTURE"' EXIT

cat > "$FIXTURE" <<'EOF'
# pid	state	etime	brief	log_age_min	last_event
1001	R+	00:42	healthy-running-brief	1	assistant
1002	SN	1-04:17	suspicious-long-cargo	15	assistant
1003	SN	2-09:33	zombie-after-result	42	result:success
1004	SN	1-15:00	zombie-after-error	25	result:error
1005	S+	00:05	healthy-just-started	0	system:init
1006	SN	1-08:00	zombie-wedge-on-init-after-result	48	system:init
EOF

output=$(bash "$SUT" --fixture "$FIXTURE" 2>&1)
rc=$?

# Expect rc=1 (zombies present)
if [[ "$rc" -eq 1 ]]; then
  pass "fixture mode exits 1 when zombies present"
else
  fail "fixture mode — expected rc=1, got rc=$rc"
fi

# Expect header line
if echo "$output" | head -1 | grep -q "^STATUS"; then
  pass "fixture output has STATUS header"
else
  fail "fixture output missing STATUS header — first line: $(echo "$output" | head -1)"
fi

# Expect 🟢healthy for pid 1001
if echo "$output" | grep -q $'🟢healthy\t1001\t'; then
  pass "1001 classified as 🟢healthy (R+ state)"
else
  fail "1001 not classified as 🟢healthy — output: $(echo "$output" | grep 1001 || echo MISSING)"
fi

# Expect 🟡suspicious for pid 1002
if echo "$output" | grep -q $'🟡suspicious\t1002\t'; then
  pass "1002 classified as 🟡suspicious (SN + log 15min silent + last=assistant)"
else
  fail "1002 not classified as 🟡suspicious — output: $(echo "$output" | grep 1002 || echo MISSING)"
fi

# Expect 🔴zombie for pid 1003
if echo "$output" | grep -q $'🔴zombie\t1003\t'; then
  pass "1003 classified as 🔴zombie (SN + log 42min silent + last=result:success)"
else
  fail "1003 not classified as 🔴zombie — output: $(echo "$output" | grep 1003 || echo MISSING)"
fi

# Expect 🔴zombie for pid 1004
if echo "$output" | grep -q $'🔴zombie\t1004\t'; then
  pass "1004 classified as 🔴zombie (SN + log 25min silent + last=result:error)"
else
  fail "1004 not classified as 🔴zombie — output: $(echo "$output" | grep 1004 || echo MISSING)"
fi

# Expect 🟢healthy for pid 1005
if echo "$output" | grep -q $'🟢healthy\t1005\t'; then
  pass "1005 classified as 🟢healthy (S+ state)"
else
  fail "1005 not classified as 🟢healthy — output: $(echo "$output" | grep 1005 || echo MISSING)"
fi

# Expect 🔴zombie for pid 1006 — PID-14468 incident pattern.
# Claude emitted result:success, then Claude Code's background-task wake
# emitted system:init to re-engage; claude wedged on the wake. Last raw
# log line was `init`, not result. Pre-followup classify() skipped this
# (terminal=0 → 🟡suspicious at best). The followup treats system:init
# as terminal-equivalent when state=SN AND log_age ≥20min — same zombie
# signature as the original.
if echo "$output" | grep -q $'🔴zombie\t1006\t'; then
  pass "1006 classified as 🔴zombie (SN + log 48min silent + last=system:init — PID-14468 pattern)"
else
  fail "1006 not classified as 🔴zombie — output: $(echo "$output" | grep 1006 || echo MISSING)"
fi

# ------------------------------------------------------------------
# Test 3: empty fixture exits 0 (no zombies present)
# ------------------------------------------------------------------
EMPTY=$(mktemp -t zombie-empty.XXXXXX)
echo "# only a comment" > "$EMPTY"
output=$(bash "$SUT" --fixture "$EMPTY" 2>&1)
rc=$?
rm -f "$EMPTY"
if [[ "$rc" -eq 0 ]]; then
  pass "empty fixture exits 0"
else
  fail "empty fixture — expected rc=0, got rc=$rc, output=$output"
fi

# ------------------------------------------------------------------
echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]] && exit 0 || exit 1
