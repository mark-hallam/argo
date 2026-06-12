#!/usr/bin/env bash
# Assertions for the worker-loop quota-sleep probe-resume + cap + signal fixes
# (fix-worker-quota-sleep-overshoot-probe-resume).
#
# Sources claude-worker-loop.sh with CLAUDE_WORKER_LOOP_SOURCE_ONLY=1 so only
# the functions load (the main `while true` loop is skipped). A PATH-injected
# `claude` stub stands in for the real CLI so the probe path is exercised
# without burning quota. Ticks are kept tiny via env so the suite runs in
# seconds.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOOP="$SCRIPT_DIR/claude-worker-loop.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# --- PATH-injected `claude` stub ---------------------------------------------
# Fails (exit 1, empty output) for the first $STUB_FAIL_COUNT invocations, then
# succeeds (prints "ok"). Invocation count persists in $STUB_COUNT_FILE.
mkdir -p "$tmp/bin"
cat > "$tmp/bin/claude" <<'STUB'
#!/usr/bin/env bash
count_file="${STUB_COUNT_FILE:?}"
n=$(( $(cat "$count_file" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$count_file"
if [ "$n" -le "${STUB_FAIL_COUNT:-0}" ]; then
  exit 1
fi
echo "ok"
STUB
chmod +x "$tmp/bin/claude"
export PATH="$tmp/bin:$PATH"

# Source-only guard inputs: the sourced script runs its top-level guards before
# the guard returns, so keep the env clean.
unset ANTHROPIC_API_KEY
export CLAUDE_WORKER_LOOP_SOURCE_ONLY=1
# shellcheck source=scripts/claude-worker-loop.sh disable=SC1090,SC1091
source "$LOOP"

fail() { echo "FAIL: $1" >&2; exit 1; }

echo "[1/4] probe fails twice then succeeds -> sleep loop resumes early on 3rd probe tick"
export STUB_COUNT_FILE="$tmp/probe-count"; : > "$STUB_COUNT_FILE"
export STUB_FAIL_COUNT=2
export QUOTA_HEARTBEAT_INTERVAL=1     # 1s ticks
export QUOTA_PROBE_INTERVAL_TICKS=1   # probe every tick
export QUOTA_PROBE_TIMEOUT_SECS=5
# REMAINING far larger than 3 ticks: if probe-resume did NOT fire, the function
# would run ~600 ticks. Early return is what makes this finish in ~3s.
start=$(date +%s)
quota_sleep_with_probe 600 "test" >"$tmp/probe.log" 2>&1 || fail "quota_sleep_with_probe returned non-zero"
elapsed=$(( $(date +%s) - start ))
probes=$(cat "$STUB_COUNT_FILE")
[ "$probes" -eq 3 ] || fail "expected exactly 3 probes (fail,fail,succeed), got $probes"
[ "$elapsed" -lt 30 ] || fail "expected early resume (~3s), took ${elapsed}s — probe-resume did not break the sleep"
grep -q "quota probe OK — resuming early" "$tmp/probe.log" || fail "missing early-resume log line"

echo "[2/4] probe disabled (interval=0) sleeps the full short duration, never probes"
export STUB_COUNT_FILE="$tmp/noprobe-count"; : > "$STUB_COUNT_FILE"
export STUB_FAIL_COUNT=0
export QUOTA_HEARTBEAT_INTERVAL=1
export QUOTA_PROBE_INTERVAL_TICKS=0   # disabled
quota_sleep_with_probe 2 "test" >/dev/null 2>&1 || fail "disabled-probe sleep returned non-zero"
probes=$(cat "$STUB_COUNT_FILE" 2>/dev/null || true); probes=${probes:-0}
[ "$probes" -eq 0 ] || fail "expected 0 probes when QUOTA_PROBE_INTERVAL_TICKS=0, got $probes"

echo "[3/4] SLEEP_SECS larger than the cap is clamped; under-cap is untouched"
export QUOTA_SLEEP_CAP_SECS=7200
[ "$(clamp_quota_sleep 79200)" -eq 7200 ] || fail "22h sleep should clamp to 7200s cap"
[ "$(clamp_quota_sleep 3600)" -eq 3600 ] || fail "1h sleep (under cap) should pass through unchanged"
[ "$(clamp_quota_sleep 7200)" -eq 7200 ] || fail "exactly-cap sleep should pass through unchanged"

echo "[4/4] SIGTERM mid-sleep exits within one tick (background-sleep + wait)"
# Run the real function in a child bash that sources the script and installs a
# trivial trap, then background a long sleep and TERM it. If the old foreground
# `sleep` were still in place the trap would not fire until the tick elapsed.
cat > "$tmp/term-case.sh" <<TERMCASE
#!/usr/bin/env bash
set -euo pipefail
export CLAUDE_WORKER_LOOP_SOURCE_ONLY=1
unset ANTHROPIC_API_KEY
source "$LOOP"
trap 'echo TRAPPED; exit 7' TERM
export QUOTA_HEARTBEAT_INTERVAL=30      # one 30s tick — far longer than our wait
export QUOTA_PROBE_INTERVAL_TICKS=0
quota_sleep_with_probe 600 "test"
echo "FELL_THROUGH"
TERMCASE
chmod +x "$tmp/term-case.sh"
bash "$tmp/term-case.sh" >"$tmp/term.log" 2>&1 &
term_pid=$!
sleep 2                                 # let it enter the 30s tick
kill -TERM "$term_pid" 2>/dev/null || true
# Wait up to 5s for it to die; old foreground sleep would survive ~30s.
died=0
for _ in $(seq 1 5); do
  if ! kill -0 "$term_pid" 2>/dev/null; then died=1; break; fi
  sleep 1
done
if [ "$died" -ne 1 ]; then
  kill -KILL "$term_pid" 2>/dev/null || true
  fail "SIGTERM did not interrupt the quota-sleep within 5s (trap deferred by foreground sleep)"
fi
wait "$term_pid" 2>/dev/null || true
grep -q "TRAPPED" "$tmp/term.log" || fail "trap did not fire mid-sleep"
grep -q "FELL_THROUGH" "$tmp/term.log" && fail "function returned normally instead of being interrupted"

echo "claude-worker-loop quota-sleep tests passed"
