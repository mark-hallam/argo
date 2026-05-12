#!/usr/bin/env bash
# Record a criterion bench baseline as JSON.
#
# Walks target/criterion/<group>/<case>/new/estimates.json (criterion
# schema: mean.point_estimate + mean.standard_error) and writes one
# target/bench-baseline-<git-sha>.json. host_fingerprint guards
# bench-compare.sh against accidental cross-host comparison.
#
# Stream: mission-bench-perf-regression-detection. Runbook:
# docs/skycot/runbook/perf-benches.md → Regression detection.
# Requires: jq.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
  cat <<'EOF'
Usage: bench-record.sh [--skip-run] [--quiet] [--output PATH] [--help]

Record a criterion bench baseline. Parses target/criterion/ and
writes a JSON snapshot of per-bench mean (ns) and standard error.

Options:
  --skip-run     Skip scripts/run-bench.sh; parse existing target/criterion/.
  --quiet        Redirect bench output to a log file.
  --output PATH  Override default (target/bench-baseline-<sha>.json).
  --help         Print this help.

Exit: 0 on success, 1 if bench failed / target/criterion empty, 2 usage.
EOF
}

SKIP_RUN=0; QUIET=0; OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-run) SKIP_RUN=1; shift ;;
    --quiet)    QUIET=1; shift ;;
    --output)   OUT="${2:?--output needs PATH}"; shift 2 ;;
    --help|-h)  usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage >&2; exit 2 ;;
  esac
done

command -v jq >/dev/null || { echo "jq required" >&2; exit 2; }

GIT_SHA="$(git rev-parse --short=12 HEAD 2>/dev/null || echo unknown)"
RECORDED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
case "$(uname -s)" in
  Darwin) MODEL="$(sysctl -n hw.model 2>/dev/null || echo unknown)" ;;
  Linux)  MODEL="$(lscpu 2>/dev/null | head -5 | tr '\n' ' ' || echo unknown)" ;;
  *)      MODEL="unknown" ;;
esac
ARCH="$(uname -sm | tr ' ' '-')"
if command -v shasum >/dev/null; then
  MODEL_HASH="$(printf '%s' "$MODEL" | shasum -a 256 | cut -c1-12)"
else
  MODEL_HASH="$(printf '%s' "$MODEL" | sha256sum | cut -c1-12)"
fi
HOST_FP="${ARCH}-${MODEL_HASH}"

if [[ $SKIP_RUN -eq 0 ]]; then
  if [[ $QUIET -eq 1 ]]; then
    LOG="$(mktemp -t bench-record.XXXXXX)"
    echo "→ running scripts/run-bench.sh (quiet, log: $LOG)"
    bash scripts/run-bench.sh >"$LOG" 2>&1 \
      || { echo "✗ run-bench.sh failed; see $LOG" >&2; exit 1; }
  else
    bash scripts/run-bench.sh
  fi
fi

if [[ ! -d target/criterion ]]; then
  echo "✗ target/criterion/ not found — run 'cargo bench' or 'scripts/run-bench.sh' first" >&2
  exit 1
fi

results="$(
  find target/criterion -type f -name estimates.json -path '*/new/estimates.json' \
    | while read -r f; do
        rel="${f#target/criterion/}"; rel="${rel%/new/estimates.json}"
        case "$rel" in report|*/report|*/change|*/change/*) continue ;; esac
        jq --arg k "${rel//\//::}" \
          '{($k): {ns: .mean.point_estimate, stderr_ns: .mean.standard_error}}' "$f"
      done | jq -s 'add // {}'
)"

if [[ "$(jq 'length' <<<"$results")" -eq 0 ]]; then
  echo "✗ no criterion estimates found under target/criterion/" >&2
  echo "  Run 'cargo bench' or 'scripts/run-bench.sh' first." >&2
  exit 1
fi

OUT="${OUT:-target/bench-baseline-${GIT_SHA}.json}"
mkdir -p "$(dirname "$OUT")"
jq -n --arg sha "$GIT_SHA" --arg ts "$RECORDED_AT" --arg fp "$HOST_FP" \
      --argjson r "$results" \
  '{git_sha:$sha, recorded_at:$ts, host_fingerprint:$fp, results:$r}' > "$OUT"

echo "✓ wrote $OUT ($(jq '.results | length' "$OUT") benches, host=$HOST_FP)"
