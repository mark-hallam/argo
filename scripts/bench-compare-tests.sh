#!/usr/bin/env bash
# Meta-test for scripts/bench-compare.sh.
# Stream: mission-bench-perf-regression-detection.
#
# Asserts: REGRESS/OK/IMPROVE classification at default 5% threshold,
# --threshold-pct suppression, cross-host guard refusal + override,
# JSON envelope schema.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
command -v jq >/dev/null || { echo "jq required" >&2; exit 2; }

TMP="$(mktemp -d -t bench-compare-tests.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
baseline="$TMP/baseline.json"; current="$TMP/current.json"

cat >"$baseline" <<'JSON'
{ "git_sha":"a", "recorded_at":"2026-05-12T00:00:00Z", "host_fingerprint":"test-host-fp",
  "results": {
    "agent_turn::p50":  {"ns":1000000,"stderr_ns":1000},
    "audit_write::seq": {"ns": 100000,"stderr_ns":1000},
    "gateway::p50":     {"ns": 500000,"stderr_ns":5000}}}
JSON
# +6% regress, +4% ok, -6% improve.
cat >"$current" <<'JSON'
{ "git_sha":"b", "recorded_at":"2026-05-12T00:01:00Z", "host_fingerprint":"test-host-fp",
  "results": {
    "agent_turn::p50":  {"ns":1060000,"stderr_ns":1000},
    "audit_write::seq": {"ns": 104000,"stderr_ns":1000},
    "gateway::p50":     {"ns": 470000,"stderr_ns":5000}}}
JSON

fail=0
ok()  { echo "  ✓ $1"; }
bad() { echo "  ✗ $1" >&2; fail=1; }

echo "▶ classification at default 5% threshold"
set +e
out="$(bash scripts/bench-compare.sh --baseline "$baseline" --current "$current" --json 2>&1)"; ec=$?
set -e
[[ $ec -eq 1 ]] && ok "exit 1 (regress)" || bad "expected exit 1, got $ec"
cls() { jq -r ".results[]|select(.name==\"$1\").classification" <<<"$out"; }
[[ "$(cls agent_turn::p50)"  = "REGRESS" ]] && ok "+6% → REGRESS"  || bad "+6% misclassified"
[[ "$(cls audit_write::seq)" = "OK"      ]] && ok "+4% → OK"       || bad "+4% misclassified"
[[ "$(cls gateway::p50)"     = "IMPROVE" ]] && ok "-6% → IMPROVE"  || bad "-6% misclassified"
sum() { jq -r ".summary.$1" <<<"$out"; }
[[ "$(sum regress)" = "1" && "$(sum improve)" = "1" && "$(sum ok)" = "1" ]] \
  && ok "summary counts {regress:1,improve:1,ok:1}" || bad "summary counts wrong"

echo "▶ --threshold-pct 10 suppresses regressions"
set +e
out="$(bash scripts/bench-compare.sh --baseline "$baseline" --current "$current" --threshold-pct 10 --json 2>&1)"; ec=$?
set -e
[[ $ec -eq 0 && "$(jq -r '.summary.regress' <<<"$out")" = "0" ]] \
  && ok "exit 0, summary.regress=0" || bad "threshold 10 failed (ec=$ec)"

echo "▶ cross-host guard"
mismatched="$TMP/mismatched.json"
jq '.host_fingerprint="another-host-fp"' "$current" > "$mismatched"
set +e
err="$(bash scripts/bench-compare.sh --baseline "$baseline" --current "$mismatched" --json 2>&1 1>/dev/null)"; ec=$?
set -e
[[ $ec -eq 2 ]] && ok "exit 2 on host mismatch" || bad "expected exit 2, got $ec"
grep -q "host_fingerprint mismatch" <<<"$err" && ok "diagnostic mentions host_fingerprint" \
  || bad "no host_fingerprint diagnostic"
grep -q "allow-cross-host"          <<<"$err" && ok "diagnostic suggests --allow-cross-host" \
  || bad "no --allow-cross-host suggestion"

echo "▶ --allow-cross-host overrides guard"
set +e
bash scripts/bench-compare.sh --baseline "$baseline" --current "$mismatched" --allow-cross-host --json >/dev/null 2>&1
ec=$?
set -e
[[ $ec -eq 1 ]] && ok "exit 1 (regress still detected with override)" \
  || bad "expected exit 1 with override, got $ec"

echo "▶ JSON envelope schema"
set +e
out="$(bash scripts/bench-compare.sh --baseline "$baseline" --current "$current" --json 2>&1)"
set -e
if jq -e '
  (.results | type == "array")
  and (.summary.regress != null) and (.summary.improve != null)
  and (.summary.ok      != null) and (.summary.threshold_pct != null)
  and (.results[0].name != null) and (.results[0].baseline_ns != null)
  and (.results[0].current_ns != null) and (.results[0].delta_pct != null)
  and (.results[0].classification != null)' >/dev/null <<<"$out"; then
  ok "envelope schema valid"
else
  bad "envelope schema invalid"
fi

if [[ $fail -eq 0 ]]; then
  echo; echo "✓ bench-compare-tests.sh: all assertions passed"; exit 0
else
  echo; echo "✗ bench-compare-tests.sh: failures above" >&2; exit 1
fi
