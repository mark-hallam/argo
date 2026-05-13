#!/usr/bin/env bash
set -euo pipefail

COVERAGE_JSON="${1:-target/llvm-cov/json/coverage.json}"
CRITICAL_CRATES=(
  "argo-store:70"
  "argo-skill:70"
  "argo-judge:70"
)

usage() {
  cat <<'USAGE'
Usage: scripts/check-coverage-threshold.sh [coverage-json]
       scripts/check-coverage-threshold.sh --self-test

Checks critical crate line coverage in cargo-llvm-cov JSON output.
USAGE
}

crate_counts() {
  local coverage_json="$1"
  local crate="$2"
  jq -r --arg crate "$crate" '
    [
      .data[]?.files[]?
      | ((.filename // "") | gsub("\\\\"; "/")) as $path
      | select(
          ($path | contains("/crates/" + $crate + "/"))
          or ($path | startswith("crates/" + $crate + "/"))
        )
      | .summary.lines
    ] as $lines
    | {
        covered: ([$lines[].covered] | add // 0),
        count: ([$lines[].count] | add // 0)
      }
    | [.covered, .count] | @tsv
  ' "$coverage_json"
}

coverage_percent() {
  local covered="$1"
  local count="$2"
  awk -v covered="$covered" -v count="$count" \
    'BEGIN { if (count == 0) printf "0.0"; else printf "%.1f", covered * 100 / count }'
}

below_floor() {
  local percent="$1"
  local floor="$2"
  awk -v percent="$percent" -v floor="$floor" 'BEGIN { exit !(percent < floor) }'
}

run_threshold_check() {
  local coverage_json="$1"
  local summary_file="${2:-}"

  if [[ ! -f "$coverage_json" ]]; then
    echo "::error::coverage JSON not found: $coverage_json" >&2
    return 1
  fi
  if ! command -v jq >/dev/null 2>&1; then
    echo "::error::jq is required to parse llvm-cov JSON" >&2
    return 1
  fi

  local output
  output="$(mktemp)"
  {
    echo "### Critical crate coverage thresholds"
    echo
    echo "| Crate | Covered lines | Total lines | Coverage | Floor | Status |"
    echo "|---|---:|---:|---:|---:|---|"
  } >"$output"

  local failed=0
  local spec crate floor counts covered count percent status
  for spec in "${CRITICAL_CRATES[@]}"; do
    crate="${spec%%:*}"
    floor="${spec##*:}"
    counts="$(crate_counts "$coverage_json" "$crate")"
    covered="${counts%%$'\t'*}"
    count="${counts##*$'\t'}"
    status="PASS"
    if [[ "$count" == "0" ]]; then
      status="FAIL"
      failed=1
      echo "::error::${crate} has no line coverage entries in ${coverage_json}" >&2
      printf "| \`%s\` | %s | %s | 0.0%% | %s%% | %s |\n" \
        "$crate" "$covered" "$count" "$floor" "$status" >>"$output"
      continue
    fi

    percent="$(coverage_percent "$covered" "$count")"
    if below_floor "$percent" "$floor"; then
      status="FAIL"
      failed=1
      echo "::error::${crate} line coverage ${percent}% is below ${floor}% floor" >&2
    fi
    printf "| \`%s\` | %s | %s | %s%% | %s%% | %s |\n" \
      "$crate" "$covered" "$count" "$percent" "$floor" "$status" >>"$output"
  done

  cat "$output"
  if [[ -n "$summary_file" ]]; then
    cat "$output" >>"$summary_file"
  fi
  rm -f "$output"
  return "$failed"
}

self_test() {
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  cat >"$tmp/pass.json" <<'JSON'
{"data":[{"files":[
  {"filename":"/repo/crates/argo-store/src/lib.rs","summary":{"lines":{"covered":70,"count":100}}},
  {"filename":"crates/argo-skill/src/lib.rs","summary":{"lines":{"covered":8,"count":10}}},
  {"filename":"/repo/crates/argo-judge/src/lib.rs","summary":{"lines":{"covered":14,"count":20}}},
  {"filename":"/repo/crates/argo-cli/src/main.rs","summary":{"lines":{"covered":1,"count":100}}}
]}]}
JSON
  cat >"$tmp/fail.json" <<'JSON'
{"data":[{"files":[
  {"filename":"/repo/crates/argo-store/src/lib.rs","summary":{"lines":{"covered":69,"count":100}}},
  {"filename":"/repo/crates/argo-skill/src/lib.rs","summary":{"lines":{"covered":7,"count":10}}},
  {"filename":"/repo/crates/argo-judge/src/lib.rs","summary":{"lines":{"covered":14,"count":20}}}
]}]}
JSON

  local summary="$tmp/summary.md"
  run_threshold_check "$tmp/pass.json" "$summary" >/dev/null
  if run_threshold_check "$tmp/fail.json" "$summary" >/dev/null 2>"$tmp/fail.err"; then
    echo "self-test expected failing fixture to fail" >&2
    return 1
  fi
  grep -F -q 'argo-store' "$summary"
  grep -q 'below 70% floor' "$tmp/fail.err"
  echo "self-test passed"
}

case "${1:-}" in
  --self-test)
    self_test
    ;;
  -h|--help)
    usage
    ;;
  *)
    run_threshold_check "$COVERAGE_JSON" "${GITHUB_STEP_SUMMARY:-}"
    ;;
esac
