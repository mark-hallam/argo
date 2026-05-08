#!/usr/bin/env bash
# scripts/profile-startup.sh — Profile `argo gateway start` cold +
# warm wall-clock under `--profile-startup`. Used by brief
# `claude-perf-startup-time` to drive cold-start under 1s on M-series.
#
# Usage: scripts/profile-startup.sh [N_RUNS]   (default N_RUNS=5)
#
# Stages:
#   1. cargo build --release -p argo-cli
#   2. one cold run against a fresh ARGO_HOME (forces migrations etc.)
#      printed with full per-phase trace
#   3. (N_RUNS - 1) warm runs against the now-migrated home; report
#      median wall-clock
set -euo pipefail

N_RUNS=${1:-5}
BIN_DIR="${CARGO_TARGET_DIR:-target}/release"
BINARY="$BIN_DIR/argo"

echo "=== argo startup profiler ==="
echo "hardware: $(uname -m) / $(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)"
echo "n_runs:   $N_RUNS (1 cold + $((N_RUNS - 1)) warm)"
echo

echo "+ cargo build --release -p argo-cli"
cargo build --release -p argo-cli 2>&1 | tail -3
[[ -x "$BINARY" ]] || { echo "error: $BINARY not built (set CARGO_TARGET_DIR?)" >&2; exit 1; }
echo

TMP_HOME=$(mktemp -d)
export ARGO_HOME="$TMP_HOME"
trap 'rm -rf "$TMP_HOME"' EXIT

# Wall-clock helper: print elapsed ms for the supplied command.
ms() {
  local s e
  s=$(perl -MTime::HiRes=time -e 'printf "%.3f", time()*1000')
  "$@" >/dev/null 2>&1 || true
  e=$(perl -MTime::HiRes=time -e 'printf "%.3f", time()*1000')
  perl -e "printf '%.0f', $e - $s"
}

echo "=== cold (first run, full per-phase trace) ==="
"$BINARY" gateway start --profile-startup 2>&1 || true
echo

cold_ms=$(ms "$BINARY" gateway start --profile-startup)
echo "cold total wall-clock (re-run for timing): ${cold_ms}ms"
echo

declare -a warm_results=()
if (( N_RUNS > 1 )); then
  echo "=== warm runs ==="
  for i in $(seq 2 "$N_RUNS"); do
    t=$(ms "$BINARY" gateway start --profile-startup)
    warm_results+=("$t")
    echo "  warm run $i: ${t}ms"
  done
fi

echo
echo "summary:"
echo "  cold:        ${cold_ms}ms (target < 1000ms)"
if (( ${#warm_results[@]} > 0 )); then
  sorted=$(printf '%s\n' "${warm_results[@]}" | sort -n)
  median_idx=$(( (${#warm_results[@]} - 1) / 2 + 1 ))
  median=$(echo "$sorted" | sed -n "${median_idx}p")
  echo "  warm median: ${median}ms (target < 500ms)"
fi
