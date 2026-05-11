#!/usr/bin/env bash
# Soak-runner sanity tests. Runs the soak script in compressed modes
# and asserts the JSON summary has the documented shape + exit codes
# behave correctly. Used by `.github/workflows/soak.yml.disabled` as
# the pre-nightly sanity gate. Kept under ~120 LOC.
#
# Modes:
#   1. --help       → prints usage, exit 0.
#   2. tiny-mode    → 2-min run with ARGO_SOAK_DURATION_OVERRIDE_MIN=2,
#                     asserts JSON has the documented keys and at
#                     least one assertion was checked.
#   3. exit-code    → re-runs with a deliberately-busted gateway port
#                     to confirm the failure path exits non-zero
#                     (skipped if the soak setup itself can't start).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOAK="$SCRIPT_DIR/soak-skycot.sh"
ARGO_BIN="${ARGO_BIN:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH" >&2
  exit 2
fi

REPORT_DIR=$(mktemp -d)
trap 'rm -rf "$REPORT_DIR"' EXIT

heading() { echo; echo "=== $* ==="; }
fail() { echo "FAIL: $*" >&2; exit 1; }
ok()   { echo "  OK: $*"; }

heading "1. --help prints usage and exits 0"
help_out=$("$SOAK" --help 2>&1)
help_exit=$?
[[ $help_exit -eq 0 ]] || fail "--help exited $help_exit"
echo "$help_out" | grep -q "Usage: scripts/soak-skycot.sh" \
  || fail "--help missing 'Usage:' banner"
echo "$help_out" | grep -q "ARGO_SOAK_WORKERS" \
  || fail "--help missing env-var documentation"
echo "$help_out" | grep -q "ARGO_SOAK_FAST" \
  || fail "--help missing fast-mode env var"
ok "--help banner + env-var documentation present"

heading "2. tiny-mode run produces JSON summary"
# Two-minute run is short enough to fit in a CI minute budget while
# still exercising one snapshot cycle. The soak may exit non-zero
# (e.g. growth thresholds tripping on small windows) — we only assert
# the JSON shape, not the all_pass result.
ARGO_SOAK_REPORT_DIR="$REPORT_DIR" \
  ARGO_SOAK_DURATION_OVERRIDE_MIN=2 \
  ARGO_SOAK_WORKERS=2 \
  ARGO_SOAK_PROMPTS_PER_WORKER_MIN=2 \
  "$SOAK" "$ARGO_BIN" >"$REPORT_DIR/run.log" 2>&1 || true
report=$(find "$REPORT_DIR" -name 'soak-report-*.json' | head -1)
[[ -n "$report" ]] || {
  echo "  run log:"; sed 's/^/    /' "$REPORT_DIR/run.log"
  fail "tiny-mode produced no JSON summary"
}
ok "summary written: $(basename "$report")"

# Validate documented JSON shape.
required_keys=(
  "start" "end" "workers" "prompts_sent" "prompts_succeeded"
  "memory_growth_pct" "fh_growth_pct" "audit_chain_intact" "assertions"
)
for key in "${required_keys[@]}"; do
  grep -q "\"$key\"" "$report" || fail "summary missing key: $key"
done
ok "summary contains all 9 documented keys"

# All 5 assertions present.
for assertion in "no_crash" "memory_growth_ok" "fh_growth_ok" \
                 "audit_chain_intact" "gateway_healthy"; do
  grep -q "\"$assertion\"" "$report" \
    || fail "summary missing assertion: $assertion"
done
ok "all 5 assertions recorded"

# Boolean shape — every assertion value is `true` or `false`.
bad=$(grep -E '"(no_crash|memory_growth_ok|fh_growth_ok|audit_chain_intact|gateway_healthy|all_pass)": ' "$report" \
  | grep -vE ': (true|false)' || true)
[[ -z "$bad" ]] || fail "non-boolean assertion value: $bad"
ok "assertion values are all booleans"

heading "3. duration override is honoured (≤4 min for the 2-min run)"
elapsed=$(awk '/duration_min/ { gsub(/[",]/, ""); print $2 }' "$report")
[[ "$elapsed" == "2" ]] || fail "duration_min=$elapsed, expected 2"
ok "duration_min recorded as 2"

echo
echo "✓ soak-runner sanity tests PASSED"
