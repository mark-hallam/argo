#!/usr/bin/env bash
# test-ledger-race.sh — self-contained verifier for with-ledger-lock.sh
# (DEC-042 Stage 1, B-1 acceptance #1 — the "zero divergence flakes over a
# fixed window" check, made hermetic so no live swarm is needed).
#
# What it proves: K concurrent processes each perform a non-atomic
# read-modify-write (read whole file → tiny sleep to widen the race window →
# append a uniquely-tagged line → write whole file back) M times against ONE
# throwaway ledger file, each RMW serialized by with-ledger-lock.sh. With the
# lock, every one of the K*M tagged lines survives. Without serialization the
# interleaved read-then-overwrite clobbers lines and tags go missing.
#
# Runs the whole experiment ROUNDS (default 3) times; passes only on
# ROUNDS/ROUNDS zero-loss.
#
# Usage:
#   bash scripts/test-ledger-race.sh                 # K=8 M=10 ROUNDS=3
#   bash scripts/test-ledger-race.sh --baseline      # ALSO run an unlocked
#                                                     # control to show the lock
#                                                     # is what saves the lines
#
# Internal RMW worker mode (not for direct use):
#   bash scripts/test-ledger-race.sh --rmw <file> <tag>
#
# Exit codes:
#   0 = all rounds zero-loss
#   1 = at least one round lost an update

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCK="$SCRIPT_DIR/with-ledger-lock.sh"

# --- internal: one non-atomic read-modify-write of <file>, appending <tag> ---
if [ "${1:-}" = "--rmw" ]; then
  file="${2:?rmw needs a file}"
  tag="${3:?rmw needs a tag}"
  content="$(cat "$file" 2>/dev/null)"
  # Widen the read→write window so an unserialized run reliably loses updates.
  sleep 0.01
  printf '%s\n%s\n' "$content" "$tag" > "$file"
  exit 0
fi

K="${LEDGER_RACE_K:-8}"
M="${LEDGER_RACE_M:-10}"
ROUNDS="${LEDGER_RACE_ROUNDS:-3}"
RUN_BASELINE=0
[ "${1:-}" = "--baseline" ] && RUN_BASELINE=1

# run_round <locked|unlocked> — returns 0 if zero-loss, 1 if any tag lost.
run_round() {
  local mode="$1"
  local workdir ledger expected found
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/ledger-race.XXXXXX")"
  ledger="$workdir/BUILD_STATUS.copy.md"
  : > "$ledger"
  # Per-round private lock dir so concurrent CI runs of this test don't share a
  # lock (and so a crashed prior round can't leave a stale lock behind).
  export LEDGER_LOCK_DIR="$workdir/lock"
  export LEDGER_LOCK_TIMEOUT=30

  local k m
  for k in $(seq 1 "$K"); do
    (
      for m in $(seq 1 "$M"); do
        local tag="tag-${k}-${m}"
        if [ "$mode" = "locked" ]; then
          bash "$LOCK" -- bash "$0" --rmw "$ledger" "$tag"
        else
          bash "$0" --rmw "$ledger" "$tag"
        fi
      done
    ) &
  done
  wait

  expected=$((K * M))
  found="$(grep -cE '^tag-[0-9]+-[0-9]+$' "$ledger" 2>/dev/null || echo 0)"
  rm -rf "$workdir"
  unset LEDGER_LOCK_DIR LEDGER_LOCK_TIMEOUT

  if [ "$found" -eq "$expected" ]; then
    echo "  [$mode] round OK — $found/$expected tags present (zero loss)"
    return 0
  fi
  echo "  [$mode] round LOST UPDATES — $found/$expected tags present ($((expected - found)) missing)" >&2
  return 1
}

echo "=== ledger-race harness: K=$K concurrent × M=$M RMW each, ROUNDS=$ROUNDS (locked) ==="
if [ ! -x "$LOCK" ] && [ ! -r "$LOCK" ]; then
  echo "FAIL: $LOCK not found" >&2
  exit 1
fi

fails=0
for r in $(seq 1 "$ROUNDS"); do
  echo "round $r/$ROUNDS:"
  run_round locked || fails=$((fails + 1))
done

if [ "$RUN_BASELINE" -eq 1 ]; then
  echo "=== control: same workload UNLOCKED (expected to lose updates) ==="
  if run_round unlocked; then
    echo "  NOTE: unlocked control did NOT lose updates this run — the race window" >&2
    echo "        may be too narrow on this machine; the locked result above still" >&2
    echo "        stands as the acceptance signal." >&2
  fi
fi

echo
if [ "$fails" -eq 0 ]; then
  echo "PASS — $ROUNDS/$ROUNDS rounds zero-loss under with-ledger-lock.sh"
  exit 0
fi
echo "FAIL — $fails/$ROUNDS rounds lost updates under the lock" >&2
exit 1
