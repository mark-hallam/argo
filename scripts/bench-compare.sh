#!/usr/bin/env bash
# Compare a current criterion run against a recorded baseline.
#
# For each bench shared between baseline and current, computes
# delta_pct = (current_ns - baseline_ns) / baseline_ns * 100 and
# classifies:
#   REGRESS  — delta > threshold
#   IMPROVE  — delta < -threshold
#   OK       — |delta| <= threshold
#
# Exit 1 iff any bench is classified REGRESS. Host_fingerprint guard
# refuses cross-host comparison unless --allow-cross-host is passed.
# Stream: mission-bench-perf-regression-detection.
#
# Requires: jq.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
  cat <<'EOF'
Usage: bench-compare.sh --baseline PATH [--current PATH]
                        [--threshold-pct N] [--allow-cross-host]
                        [--json] [--help]

Compare a current criterion run against a recorded baseline.

Options:
  --baseline PATH       Baseline JSON (from bench-record.sh).
  --current PATH        Current-run JSON. If omitted, materialized
                        on the fly from target/criterion/ (no bench
                        re-run; assumes 'cargo bench' just ran).
  --threshold-pct N     Regression threshold percent (default 5).
  --allow-cross-host    Permit comparison across mismatched
                        host_fingerprint values.
  --json                Emit JSON envelope on stdout instead of
                        human-readable text.
  --help                Print this help.

Exit codes:
  0 — no regressions (improvements + ok cases allowed)
  1 — at least one bench classified REGRESS
  2 — usage / IO error / cross-host refused
EOF
}

BASELINE=""
CURRENT=""
THRESH=5
ALLOW_CROSS=0
JSON=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --baseline)         BASELINE="${2:?--baseline needs PATH}"; shift 2 ;;
    --current)          CURRENT="${2:?--current needs PATH}"; shift 2 ;;
    --threshold-pct)    THRESH="${2:?--threshold-pct needs N}"; shift 2 ;;
    --allow-cross-host) ALLOW_CROSS=1; shift ;;
    --json)             JSON=1; shift ;;
    --help|-h)          usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -n "$BASELINE" ]] || { echo "--baseline required" >&2; usage >&2; exit 2; }
[[ -f "$BASELINE" ]] || { echo "baseline not found: $BASELINE" >&2; exit 2; }
command -v jq >/dev/null || { echo "jq required" >&2; exit 2; }

CURRENT_TMP=""
cleanup() { [[ -n "$CURRENT_TMP" ]] && rm -f "$CURRENT_TMP"; return 0; }
trap cleanup EXIT

if [[ -z "$CURRENT" ]]; then
  CURRENT_TMP="$(mktemp -t bench-current.XXXXXX)"
  bash scripts/bench-record.sh --skip-run --output "$CURRENT_TMP" >/dev/null
  CURRENT="$CURRENT_TMP"
fi
[[ -f "$CURRENT" ]] || { echo "current not found: $CURRENT" >&2; exit 2; }

base_fp="$(jq -r '.host_fingerprint // ""' "$BASELINE")"
cur_fp="$(jq -r '.host_fingerprint // ""' "$CURRENT")"
if [[ -n "$base_fp" && -n "$cur_fp" && "$base_fp" != "$cur_fp" ]]; then
  if [[ $ALLOW_CROSS -eq 0 ]]; then
    echo "✗ host_fingerprint mismatch:" >&2
    echo "    baseline=$base_fp" >&2
    echo "    current =$cur_fp" >&2
    echo "  Re-record on this host, or pass --allow-cross-host." >&2
    exit 2
  fi
fi

envelope="$(
  jq -n \
    --slurpfile b "$BASELINE" \
    --slurpfile c "$CURRENT" \
    --argjson t "$THRESH" '
    ($b[0].results) as $br | ($c[0].results) as $cr |
    [ ($br | keys[]) as $k
      | select($cr[$k] != null and $br[$k].ns > 0)
      | { name:$k,
          baseline_ns: $br[$k].ns,
          current_ns:  $cr[$k].ns,
          delta_pct:   (($cr[$k].ns - $br[$k].ns) / $br[$k].ns * 100) }
      | . + { classification:
              (if   .delta_pct >  $t      then "REGRESS"
               elif .delta_pct < (-$t)    then "IMPROVE"
               else                            "OK"
               end) } ] as $results |
    { results: $results,
      summary: {
        threshold_pct: $t,
        regress: ($results | map(select(.classification=="REGRESS")) | length),
        improve: ($results | map(select(.classification=="IMPROVE")) | length),
        ok:      ($results | map(select(.classification=="OK"))      | length),
        missing_in_current: (($br | keys) - ($cr | keys)),
        new_in_current:     (($cr | keys) - ($br | keys))
      } }'
)"

regress_count="$(jq '.summary.regress' <<<"$envelope")"

if [[ $JSON -eq 1 ]]; then
  echo "$envelope" | jq .
else
  echo "$envelope" | jq -r '
    def fmt:
      if . >= 1000000 then "\(((. / 1000000) * 100 | round) / 100)ms"
      elif . >= 1000  then "\(. / 1000 | floor)µs"
      else                  "\(. | floor)ns"
      end;
    .results[]
    | "\(.name)\t\(.baseline_ns | fmt) → \(.current_ns | fmt)\t\(if .delta_pct >= 0 then "+" else "" end)\((.delta_pct * 10 | round) / 10)%\t\(.classification)\(if .classification=="REGRESS" then " ⚠️" else "" end)"
  ' | { command -v column >/dev/null && column -t -s $'\t' || cat; }
  echo
  echo "$envelope" | jq -r '
    "\(.summary.regress) regressions, \(.summary.improve) improvements, \(.summary.ok) ok (threshold ±\(.summary.threshold_pct)%)."'
  missing="$(jq -r '.summary.missing_in_current | join(", ")' <<<"$envelope")"
  added="$(jq -r '.summary.new_in_current | join(", ")' <<<"$envelope")"
  [[ -n "$missing" ]] && echo "Missing in current: $missing"
  [[ -n "$added"   ]] && echo "New in current (not in baseline): $added"
fi

if [[ "$regress_count" -gt 0 ]]; then exit 1; fi
exit 0
