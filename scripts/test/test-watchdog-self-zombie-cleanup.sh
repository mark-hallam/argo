#!/usr/bin/env bash
# Hermetic test for check-zombie-workers.sh watchdog-owned zsh cleanup.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_SCRIPTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SUT="$REPO_SCRIPTS_DIR/check-zombie-workers.sh"

if [[ ! -x "$SUT" ]]; then
  echo "FAIL: $SUT is not executable" >&2
  exit 1
fi

ZSH_BIN="$(command -v zsh || true)"
if [[ -z "$ZSH_BIN" ]]; then
  echo "SKIP: zsh not available"
  exit 0
fi

TMP_DIR="$(mktemp -d)"
SHARED="$TMP_DIR/SHARED.md"
TARGET_PID=""

cleanup() {
  if [[ -n "${TARGET_PID:-}" ]]; then
    kill -KILL "$TARGET_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

pass() { echo "PASS: $*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

echo "# shared" > "$SHARED"

"$ZSH_BIN" -c 'pgrep -P "$$" >/dev/null 2>&1 || true; : "claude --dangerously-skip-permissions"; while true; do sleep 600; done' &
TARGET_PID=$!

sleep 3
if ! kill -0 "$TARGET_PID" 2>/dev/null; then
  fail "fixture process died before cleanup could run"
fi

cmd="$(ps -o command= -p "$TARGET_PID" 2>/dev/null || true)"
case "$cmd" in
  *"zsh -c"*pgrep*claude*"--dangerously-skip-permissions"*) pass "fixture command matches watchdog pattern" ;;
  *) fail "fixture command did not match watchdog pattern: $cmd" ;;
esac

WATCHDOG_SELF_CLEANUP_THRESHOLD_SECONDS=1 \
WATCHDOG_SELF_CLEANUP_SHARED_PATH="$SHARED" \
  bash "$SUT" --self-cleanup-only >/tmp/watchdog-self-cleanup-test.out 2>/tmp/watchdog-self-cleanup-test.err
rc=$?
[[ "$rc" -eq 0 ]] || fail "cleanup command exited rc=$rc: $(cat /tmp/watchdog-self-cleanup-test.err)"

dead=0
for _ in 1 2 3 4 5; do
  state="$(ps -o state= -p "$TARGET_PID" 2>/dev/null | tr -d ' ')"
  if [[ -z "$state" || "$state" == Z* ]]; then
    dead=1
    break
  fi
  sleep 1
done

if [[ "$dead" -ne 1 ]]; then
  fail "fixture zsh process still alive after cleanup"
fi
wait "$TARGET_PID" 2>/dev/null || true
TARGET_PID=""
pass "fixture zsh process was killed"

if grep -q '\[watchdog-self-cleanup\].*signal=' "$SHARED"; then
  pass "SHARED.md cleanup line emitted"
else
  fail "missing [watchdog-self-cleanup] line in SHARED.md: $(cat "$SHARED")"
fi

echo "Results: all passed"
