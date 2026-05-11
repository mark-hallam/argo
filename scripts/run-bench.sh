#!/usr/bin/env bash
# Run the 5 criterion perf benches sequentially, dump criterion's
# HTML report under `target/criterion/`, and summarize per-bench
# elapsed wall-clock. Stream `mission-criterion-load-tests`.
#
# Each bench takes 2-5 min on a warm laptop; the full run is
# typically 15-25 min. Re-run is cheap once `target/` is warm.
#
# Usage:
#   scripts/run-bench.sh             # run all 5 benches
#   scripts/run-bench.sh --baseline  # save criterion baseline 'main'
#
# Output: stdout summary line per bench plus the full criterion
# HTML report under `target/criterion/`. Operators read the report
# alongside the runbook (`docs/skycot/runbook/perf-benches.md`)
# to judge whether a regression is real or noise.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

BASELINE_FLAG=""
if [[ "${1:-}" == "--baseline" ]]; then
  BASELINE_FLAG="-- --save-baseline main"
  echo "→ saving 'main' baseline this run"
fi

BENCHES=(
  "argo-runtime:agent_turn"
  "argo-audit:audit_write"
  "argo-cli:gateway_request"
  "argo-store:memory_store"
  "argo-skill:select_variant"
)

summary_lines=()
overall_start=$(date +%s)

for entry in "${BENCHES[@]}"; do
  crate="${entry%%:*}"
  bench="${entry##*:}"
  echo
  echo "════════════════════════════════════════════════════════════"
  echo "▶ ${crate} :: ${bench}"
  echo "════════════════════════════════════════════════════════════"
  bench_start=$(date +%s)
  # shellcheck disable=SC2086
  if cargo bench -p "$crate" --bench "$bench" $BASELINE_FLAG; then
    elapsed=$(( $(date +%s) - bench_start ))
    summary_lines+=("  ✓ ${crate}::${bench} (${elapsed}s)")
  else
    elapsed=$(( $(date +%s) - bench_start ))
    summary_lines+=("  ✗ ${crate}::${bench} FAILED (${elapsed}s)")
  fi
done

overall_elapsed=$(( $(date +%s) - overall_start ))

echo
echo "════════════════════════════════════════════════════════════"
echo "Summary"
echo "════════════════════════════════════════════════════════════"
for line in "${summary_lines[@]}"; do
  echo "$line"
done
echo
echo "Total: ${overall_elapsed}s"
echo "HTML report: target/criterion/report/index.html"
echo "Runbook:     docs/skycot/runbook/perf-benches.md"
