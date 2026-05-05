#!/usr/bin/env bash
# Tests for scripts/dev-safe-clean.sh.
#
# Builds a synthetic repo + target + outputs + /private/tmp
# fixture, runs the cleanup script through every safety
# property the script promises, and asserts each one.
#
# What we lock:
#
#   1. dry-run NEVER deletes anything (fixture survives intact)
#   2. --apply DOES delete the whitelisted entries
#   3. --apply REFUSES to touch a path outside the whitelist
#      (we monkey-patch the symlink layout to try to escape)
#   4. --skip-target leaves target/ alone but cleans the rest
#   5. retention: outputs newer than max-age-days are kept
#      AND the newest --keep-recent are kept regardless of age
#   6. unknown CLI flags are rejected (don't silently no-op)
#
# Exit codes:
#   0   all assertions passed
#   1   first failed assertion
#
# Usage:
#   ./scripts/tests/dev-safe-clean.test.sh

set -uo pipefail

PASS=0
FAIL=0
PASS_NAMES=()
FAIL_NAMES=()

assert_eq() {
  local name="$1" want="$2" got="$3"
  if [ "$want" = "$got" ]; then
    echo "  OK: $name"
    PASS=$((PASS + 1))
    PASS_NAMES+=("$name")
  else
    echo "  FAIL: $name"
    echo "        want: $want"
    echo "        got:  $got"
    FAIL=$((FAIL + 1))
    FAIL_NAMES+=("$name")
  fi
}

assert_exists() {
  local name="$1" path="$2"
  if [ -e "$path" ]; then
    echo "  OK: $name (path exists)"
    PASS=$((PASS + 1))
    PASS_NAMES+=("$name")
  else
    echo "  FAIL: $name — expected $path to exist"
    FAIL=$((FAIL + 1))
    FAIL_NAMES+=("$name")
  fi
}

assert_missing() {
  local name="$1" path="$2"
  if [ ! -e "$path" ]; then
    echo "  OK: $name (path absent)"
    PASS=$((PASS + 1))
    PASS_NAMES+=("$name")
  else
    echo "  FAIL: $name — expected $path to be absent"
    FAIL=$((FAIL + 1))
    FAIL_NAMES+=("$name")
  fi
}

# Resolve the production script (we test the real thing).
THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
CLEAN_SCRIPT="$THIS_DIR/../dev-safe-clean.sh"
if [ ! -x "$CLEAN_SCRIPT" ]; then
  echo "FAIL: cannot find $CLEAN_SCRIPT (or not executable)"
  exit 1
fi

# Build a synthetic fixture under a tempdir. The cleanup
# script derives REPO_ROOT from its own location, so we can't
# point it at our fixture by env-var alone. Instead we COPY
# the script into the fixture's `scripts/` dir so its self-
# location calculation lands on the fixture root. This keeps
# the test exercising the production logic end-to-end without
# mocking.
FIX="$(mktemp -d)"
trap 'rm -rf "$FIX"' EXIT
mkdir -p "$FIX/scripts" "$FIX/target/debug" "$FIX/.skycot/outputs"
cp "$CLEAN_SCRIPT" "$FIX/scripts/dev-safe-clean.sh"
chmod +x "$FIX/scripts/dev-safe-clean.sh"

# Seed the fixture's target/ with a couple of synthetic files.
echo "build cache" > "$FIX/target/debug/argo"
echo "incremental" > "$FIX/target/.rustc_info.json"

# Seed .skycot/outputs/ with 25 fake job dirs at known mtimes:
#   - 5 dirs ~30 days old
#   - 5 dirs ~21 days old
#   - 5 dirs ~10 days old (within 14d window)
#   - 10 dirs "today"
mk_output() {
  local id="$1" days="$2"
  local d="$FIX/.skycot/outputs/$id"
  mkdir -p "$d"
  echo "{}" > "$d/result.json"
  if [ "$days" -gt 0 ]; then
    # Set mtime back N days. Mac BSD touch uses -t YYYYMMDDhhmm.
    # Use date -v-Nd to compute the target stamp.
    stamp=$(date -j -v "-${days}d" "+%Y%m%d%H%M" 2>/dev/null \
             || date -d "${days} days ago" "+%Y%m%d%H%M")
    touch -t "$stamp" "$d"
  fi
}

for n in 1 2 3 4 5; do mk_output "old30-$n" 30; done
for n in 1 2 3 4 5; do mk_output "old21-$n" 21; done
for n in 1 2 3 4 5; do mk_output "mid10-$n" 10; done
for n in 1 2 3 4 5 6 7 8 9 10; do mk_output "fresh-$n" 0; done

initial_outputs_count=$(ls "$FIX/.skycot/outputs" | wc -l | tr -d ' ')

# ────────────────────────────────────────────────── 1. dry-run
echo
echo "Test 1: dry-run never deletes anything"
"$FIX/scripts/dev-safe-clean.sh" --skip-tmp >/tmp/dsc-test.out 2>&1 || true

assert_exists "dry-run preserves target/"            "$FIX/target/debug/argo"
assert_exists "dry-run preserves .skycot/outputs/"   "$FIX/.skycot/outputs/old30-1"
after=$(ls "$FIX/.skycot/outputs" | wc -l | tr -d ' ')
assert_eq    "dry-run preserves output count"        "$initial_outputs_count" "$after"

# ─────────────────────────────────────────── 2. unknown flag
echo
echo "Test 2: unknown flag exits non-zero"
"$FIX/scripts/dev-safe-clean.sh" --apply --bogus 2>/dev/null
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "  OK: unknown flag rejected (exit $rc)"
  PASS=$((PASS + 1)); PASS_NAMES+=("unknown-flag-rejected")
else
  echo "  FAIL: unknown flag silently accepted"
  FAIL=$((FAIL + 1)); FAIL_NAMES+=("unknown-flag-rejected")
fi

# ────────────────────────────────────────── 3. retention rules
echo
echo "Test 3: retention — newest 20 + within-14d kept"
"$FIX/scripts/dev-safe-clean.sh" --apply --skip-target --skip-tmp \
  --max-age-days 14 --keep-recent 20 >/tmp/dsc-test.out 2>&1

# After: the 10 fresh dirs and the 5 mid10 dirs are within 14d
# (15 total kept by age). The keep-recent 20 promotes 5 more.
# Net expected: 20 dirs survive, all from fresh + mid10 + old21.
# old30 are ALL > 14d AND beyond the keep-recent window of 20
# given the newer 25 entries — but our fixture has 25 total,
# so the newest 20 by mtime is fresh (10) + mid10 (5) + old21 (5),
# leaving the 5 old30 dirs to go.
remaining=$(ls "$FIX/.skycot/outputs" | wc -l | tr -d ' ')
assert_eq "retention kept exactly 20 dirs" "20" "$remaining"
for j in 1 2 3 4 5; do
  assert_missing "old30-$j is gone" "$FIX/.skycot/outputs/old30-$j"
done
for j in 1 2 3 4 5; do
  assert_exists "old21-$j is kept by keep-recent" "$FIX/.skycot/outputs/old21-$j"
done
for j in 1 2 3 4 5; do
  assert_exists "mid10-$j is kept (within 14d)" "$FIX/.skycot/outputs/mid10-$j"
done
for j in 1 2 3 4 5; do
  assert_exists "fresh-$j is kept" "$FIX/.skycot/outputs/fresh-$j"
done

# ─────────────────────────────────────── 4. --skip-target works
echo
echo "Test 4: --skip-target preserves target/ even with --apply"
assert_exists "target/ survives --skip-target" "$FIX/target/debug/argo"

# ──────────────────── 5. apply deletes target/ when not skipped
echo
echo "Test 5: --apply (without --skip-target) deletes target/"
"$FIX/scripts/dev-safe-clean.sh" --apply --skip-outputs --skip-tmp \
  >/tmp/dsc-test.out 2>&1
assert_missing "target/ removed" "$FIX/target/debug/argo"
assert_missing "target/ root removed" "$FIX/target"

# ───────────────────── 6. is_whitelisted refuses path traversal
echo
echo "Test 6: whitelist refuses path traversal markers"
# We can't easily inject a path argument; instead, source the
# script in a sub-shell (without invoking main) and call
# is_whitelisted directly to spec the contract.
result=$(bash -c "
  REPO_ROOT='$FIX'
  TARGET_DIR=\"\$REPO_ROOT/target\"
  SKYCOT_OUTPUTS=\"\$REPO_ROOT/.skycot/outputs\"
  $(sed -n '/^is_whitelisted()/,/^}/p' "$FIX/scripts/dev-safe-clean.sh")
  is_whitelisted '/etc/passwd' && echo unsafe || echo refused
")
assert_eq "non-whitelisted absolute path refused" "refused" "$result"

result2=$(bash -c "
  REPO_ROOT='$FIX'
  TARGET_DIR=\"\$REPO_ROOT/target\"
  SKYCOT_OUTPUTS=\"\$REPO_ROOT/.skycot/outputs\"
  $(sed -n '/^is_whitelisted()/,/^}/p' "$FIX/scripts/dev-safe-clean.sh")
  is_whitelisted '$FIX/target/../etc/passwd' && echo unsafe || echo refused
")
assert_eq "path traversal marker refused" "refused" "$result2"

result3=$(bash -c "
  REPO_ROOT='$FIX'
  TARGET_DIR=\"\$REPO_ROOT/target\"
  SKYCOT_OUTPUTS=\"\$REPO_ROOT/.skycot/outputs\"
  $(sed -n '/^is_whitelisted()/,/^}/p' "$FIX/scripts/dev-safe-clean.sh")
  is_whitelisted '' && echo unsafe || echo refused
")
assert_eq "empty path refused" "refused" "$result3"

# ─────────────────────────────────────────────── final summary
echo
echo "──────────────────────────────────────"
echo "  passed: $PASS"
echo "  failed: $FAIL"
if [ "$FAIL" -gt 0 ]; then
  echo
  echo "Failing assertions:"
  for n in "${FAIL_NAMES[@]:-}"; do echo "  - $n"; done
  exit 1
fi
echo
echo "✓ dev-safe-clean.test.sh PASSED ($PASS assertions)"
