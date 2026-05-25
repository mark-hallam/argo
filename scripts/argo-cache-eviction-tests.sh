#!/usr/bin/env bash
# argo-cache-eviction-tests.sh — smoke tests for argo-cache-eviction.sh.
#
# The brief asks for 3 cases:
#   (a) under threshold → no-op
#   (b) over threshold → evict oldest-first, preserve newest 5
#   (c) lockfile present → no-op + log skip
#
# Plus two extras worth pinning:
#   (d) active worktree shields its target dir from eviction
#   (e) dry-run (default) prints what would be evicted without deleting
#
# All tests run against a scratch /tmp-shaped root and a per-run
# lockfile, so the live /tmp/argo-cache-eviction.lock is never touched.
#
# Usage: bash scripts/argo-cache-eviction-tests.sh
# Exit:  0 all pass / 1 any fail

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SUT="$SCRIPT_DIR/argo-cache-eviction.sh"
if [[ ! -x "$SUT" ]]; then
  echo "FAIL: $SUT not executable" >&2
  exit 1
fi

PASS=0; FAIL=0
pass() { echo "PASS: $*"; PASS=$((PASS + 1)); }
fail() { echo "FAIL: $*" >&2; FAIL=$((FAIL + 1)); }

ROOT=$(mktemp -d -t argo-cache-evict.XXXXXX)
LOCK="$ROOT/lockfile"
trap 'rm -rf "$ROOT"' EXIT

# Portable epoch → "YYYYMMDDhhmm" for `touch -t`.
epoch_to_stamp() {
  local e="$1"
  date -r "$e" +%Y%m%d%H%M 2>/dev/null || date -d "@$e" +%Y%m%d%H%M
}

# Make a target dir with a small file and an explicit mtime.
# Args: relative-name size_kb age_sec
make_target() {
  local name="$1" size_kb="$2" age_sec="$3"
  local dir="$ROOT/$name"
  mkdir -p "$dir"
  # Write a small file so `du -sk` reports non-zero.
  dd if=/dev/zero of="$dir/blob" bs=1024 count="$size_kb" 2>/dev/null
  local epoch=$(( $(date +%s) - age_sec ))
  touch -t "$(epoch_to_stamp "$epoch")" "$dir"
}

reset_root() {
  rm -rf "$ROOT"/*  "$LOCK" 2>/dev/null
}

# ----------------------------------------------------------------------
# (a) under threshold → no-op
# ----------------------------------------------------------------------
reset_root
make_target "brief-a-target" 10 100
output=$(ARGO_CACHE_EVICTION_GB_THRESHOLD=999 \
         bash "$SUT" --root "$ROOT" --lockfile "$LOCK" --apply 2>&1)
rc=$?
if [[ "$rc" -eq 0 ]] && echo "$output" | grep -q "OK: under threshold"; then
  pass "(a) under threshold returns OK"
else
  fail "(a) under threshold — rc=$rc, output=$output"
fi
if [[ -d "$ROOT/brief-a-target" ]]; then
  pass "(a) brief-a-target preserved"
else
  fail "(a) brief-a-target was deleted under threshold"
fi

# ----------------------------------------------------------------------
# (b) over threshold → evict oldest-first, preserve newest 5
# ----------------------------------------------------------------------
reset_root
# 7 dirs. Use (8 - i) * 100 so b1 is oldest, b7 is newest.
for i in 1 2 3 4 5 6 7; do
  age=$(( (8 - i) * 100 ))
  make_target "brief-b${i}-target" 10 "$age"
done
output=$(ARGO_CACHE_EVICTION_GB_THRESHOLD=0 \
         ARGO_CACHE_EVICTION_RETAIN_NEWEST=5 \
         bash "$SUT" --root "$ROOT" --lockfile "$LOCK" --apply 2>&1)
rc=$?
if [[ "$rc" -eq 0 ]]; then
  pass "(b) over-threshold apply returns 0"
else
  fail "(b) over-threshold apply — rc=$rc, output=$output"
fi
# Oldest 2 (b1, b2) should be gone; newest 5 (b3..b7) preserved.
if [[ ! -d "$ROOT/brief-b1-target" ]] && [[ ! -d "$ROOT/brief-b2-target" ]]; then
  pass "(b) oldest 2 evicted (b1, b2)"
else
  fail "(b) expected b1+b2 evicted; b1=$([[ -d $ROOT/brief-b1-target ]] && echo present || echo gone) b2=$([[ -d $ROOT/brief-b2-target ]] && echo present || echo gone)"
fi
all_kept=1
for i in 3 4 5 6 7; do
  [[ -d "$ROOT/brief-b${i}-target" ]] || { all_kept=0; break; }
done
if [[ "$all_kept" -eq 1 ]]; then
  pass "(b) newest 5 preserved (b3..b7)"
else
  fail "(b) one of newest 5 was wrongly evicted; output=$output"
fi

# ----------------------------------------------------------------------
# (c) lockfile present → no-op + exit 3
# ----------------------------------------------------------------------
reset_root
echo $$ > "$LOCK"
output=$(bash "$SUT" --root "$ROOT" --lockfile "$LOCK" 2>&1)
rc=$?
rm -f "$LOCK"
if [[ "$rc" -eq 3 ]] && echo "$output" | grep -q "SKIP: lockfile"; then
  pass "(c) held lockfile exits 3 with SKIP message"
else
  fail "(c) lockfile — expected rc=3 + SKIP, got rc=$rc, output=$output"
fi

# ----------------------------------------------------------------------
# (d) active worktree shields its target dir
# ----------------------------------------------------------------------
reset_root
# Two dirs over threshold; one has a sibling worktree, the other doesn't.
make_target "brief-d-active-target" 10 9999
make_target "brief-d-stale-target"  10 9998
mkdir -p "$ROOT/brief-d-active"      # the "worktree" presence
output=$(ARGO_CACHE_EVICTION_GB_THRESHOLD=0 \
         ARGO_CACHE_EVICTION_RETAIN_NEWEST=0 \
         bash "$SUT" --root "$ROOT" --lockfile "$LOCK" --apply 2>&1)
rc=$?
if [[ "$rc" -eq 0 ]] && [[ -d "$ROOT/brief-d-active-target" ]] && [[ ! -d "$ROOT/brief-d-stale-target" ]]; then
  pass "(d) active-worktree target preserved, stale evicted"
else
  fail "(d) shielding — rc=$rc active-present=$([[ -d $ROOT/brief-d-active-target ]] && echo y || echo n) stale-present=$([[ -d $ROOT/brief-d-stale-target ]] && echo y || echo n) output=$output"
fi

# ----------------------------------------------------------------------
# (e) dry-run (default) prints DRY-RUN lines but deletes nothing
# ----------------------------------------------------------------------
reset_root
make_target "brief-e-target" 10 5000
output=$(ARGO_CACHE_EVICTION_GB_THRESHOLD=0 \
         ARGO_CACHE_EVICTION_RETAIN_NEWEST=0 \
         bash "$SUT" --root "$ROOT" --lockfile "$LOCK" 2>&1)
rc=$?
if [[ "$rc" -eq 0 ]] && echo "$output" | grep -q "DRY-RUN: would evict" && [[ -d "$ROOT/brief-e-target" ]]; then
  pass "(e) dry-run mode (default) logs DRY-RUN and preserves dirs"
else
  fail "(e) dry-run — rc=$rc dir-present=$([[ -d $ROOT/brief-e-target ]] && echo y || echo n) output=$output"
fi

# ----------------------------------------------------------------------
echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]] && exit 0 || exit 1
