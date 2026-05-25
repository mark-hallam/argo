#!/usr/bin/env bash
# Smoke tests for worker-supervisor wait-loop detection.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUPERVISOR="$SCRIPT_DIR/worker-supervisor.sh"

export ARGO_SUPERVISOR_SOURCE_ONLY=1
# shellcheck source=scripts/worker-supervisor.sh
source "$SUPERVISOR"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

make_ndjson_path() {
  local brief="$1"
  local age_secs="$2"
  python3 - "$tmp" "$brief" "$age_secs" <<'PY'
import datetime
import pathlib
import sys

root, brief, age_secs = sys.argv[1:4]
stamp = datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(seconds=int(age_secs))
print(pathlib.Path(root) / f"{stamp.strftime('%Y%m%dT%H%M%SZ')}-{brief}.ndjson")
PY
}

assert_alive() {
  local pid="$1"
  local label="$2"
  if ! kill -0 "$pid" 2>/dev/null; then
    echo "FAIL: $label" >&2
    exit 1
  fi
}

assert_dead_soon() {
  local pid="$1"
  local label="$2"
  for _ in $(seq 1 10); do
    if ! kill -0 "$pid" 2>/dev/null; then
      return 0
    fi
    sleep 1
  done
  echo "FAIL: $label" >&2
  kill "$pid" 2>/dev/null || true
  exit 1
}

echo "[1/3] one genuine wait followed by a shipping result is not killed"
WAIT_LOOP_MIN_ETIME_SECS=7200
WAIT_LOOP_MIN_CONSECUTIVE=3
one_wait_log="$(make_ndjson_path genuine-wait-then-ships 10800)"
cat > "$one_wait_log" <<'EOF'
{"type":"result","subtype":"success","result":"Waiting for clippy/tests to finish."}
{"type":"assistant","message":{"content":[{"type":"text","text":"tests passed"}]}}
{"type":"result","subtype":"success","result":"Opened PR #123 and updated BUILD_STATUS.md."}
EOF
if wait_loop_should_fire "$one_wait_log"; then
  echo "FAIL: one-wait shipping log should not fire" >&2
  exit 1
fi

echo "[2/3] three consecutive stale no-progress results kill the worker"
export WAIT_LOOP_MIN_ETIME_SECS=1
export WAIT_LOOP_MIN_CONSECUTIVE=3
export WAIT_LOOP_WATCHDOG_INTERVAL_SECS=1
export WAIT_LOOP_RELEASE_CLAIM=0
export WAIT_LOOP_WATCHDOG_ARTIFACT="$tmp/wait-loop-artifact"
stale_log="$(make_ndjson_path stale-wait-loop 10)"
cat > "$stale_log" <<'EOF'
{"type":"result","subtype":"success","result":"Waiting for clippy/tests to finish."}
{"type":"result","subtype":"success","result":""}
{"type":"result","subtype":"success","result":"Still waiting for tests."}
{"type":"result","subtype":"success","result":"Monitor is armed; waiting for cargo."}
EOF
slot_log="$tmp/slot.log"
echo "[$(date +%H:%M:%S)] Log: $stale_log" > "$slot_log"
sleep 1000 &
worker_pid=$!
monitor_wait_loop_watchdog "$worker_pid" "$slot_log" &
watchdog_pid=$!
assert_dead_soon "$worker_pid" "wait-loop watchdog did not kill stale worker"
wait "$watchdog_pid" 2>/dev/null || true
grep -q "brief=stale-wait-loop" "$WAIT_LOOP_WATCHDOG_ARTIFACT"

echo "[3/3] missing ndjson log is a no-op"
missing_slot_log="$tmp/missing-slot.log"
echo "[$(date +%H:%M:%S)] Log: $tmp/missing.ndjson" > "$missing_slot_log"
sleep 1000 &
missing_pid=$!
monitor_wait_loop_watchdog "$missing_pid" "$missing_slot_log" &
missing_watchdog_pid=$!
sleep 2
assert_alive "$missing_pid" "missing ndjson log should not kill worker"
kill "$missing_pid" 2>/dev/null || true
wait "$missing_pid" 2>/dev/null || true
kill "$missing_watchdog_pid" 2>/dev/null || true
wait "$missing_watchdog_pid" 2>/dev/null || true

echo "worker-supervisor wait-loop tests passed"
