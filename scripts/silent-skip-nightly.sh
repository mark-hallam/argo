#!/usr/bin/env bash
# SKYCOT nightly silent-skip integration runner. Discovers integration
# tests gated on env-var presence (tests `return` early when a
# credential is missing) and runs each in its own `cargo test --test`
# invocation so one hang doesn't block the rest. Aggregates to
# `target/silent-skip-report-<ts>.json`. Driven by
# `.github/workflows/silent-skip-nightly.yml.disabled`; operators run
# `--local` for runner-shape debugging. Runbook:
# `docs/skycot/runbook/silent-skip-nightly.md`.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TIMEOUT_SECS="${ARGO_SILENT_SKIP_TIMEOUT_SECS:-300}"
REPORT_DIR="${ARGO_SILENT_SKIP_REPORT_DIR:-$REPO_ROOT/target}"

MODE="run"
LOCAL=0

usage() {
  cat <<'USAGE'
Usage: scripts/silent-skip-nightly.sh [flags]

Discover + run silent-skip integration tests across the workspace.
Each test target runs in its own `cargo test --test` invocation with
a per-target timeout. Aggregates results to a JSON summary.

Flags:
  --help, -h         Print this usage and exit 0.
  --discover-only    Print discovered (crate, test_target) pairs and
                     exit 0 without running.
  --local            Local mode. Discovery + run against the
                     operator's environment (no GH secrets — most
                     tests silently skip). Useful for runner debug.

Env vars:
  ARGO_SILENT_SKIP_TIMEOUT_SECS   Per-target timeout    (default 300)
  ARGO_SILENT_SKIP_REPORT_DIR     Summary dir           (default target/)

Exit:
  0 = run completed (regardless of aggregate); summary written.
  2 = setup/discovery failure (no tests found, cargo missing, …).

Discovery: files matching `crates/*/tests/*.rs` with names containing
`_smoke|smoke_|_integration|integration_|_live|live_`. Each file is
one cargo `--test` target. Function bodies are not inspected; tests
that don't follow the file-naming convention are missed (see runbook).
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)         usage; exit 0 ;;
    --discover-only)   MODE="discover"; shift ;;
    --local)           LOCAL=1; shift ;;
    *)                 echo "FAIL: unknown flag: $1" >&2; usage >&2; exit 2 ;;
  esac
done

# ---- discovery ----------------------------------------------------
# Bash-3.2 compatible (no readarray) — macOS default.
TMP_PARENT=$(mktemp -d)
trap 'rm -rf "$TMP_PARENT"' EXIT
DISCOVERY_FILE="$TMP_PARENT/discovered.txt"

cd "$REPO_ROOT" || { echo "FAIL: cannot cd to $REPO_ROOT" >&2; exit 2; }
find crates -type f -name '*.rs' -path '*/tests/*' \
  \( -name '*_smoke*' -o -name 'smoke_*' \
     -o -name '*_integration*' -o -name 'integration_*' \
     -o -name '*_live*' -o -name 'live_*' \) 2>/dev/null \
  | sort -u \
  | awk -F/ '{ print $2 " " $4 " " $0 }' \
  | sed 's/\.rs / /' \
  >"$DISCOVERY_FILE"
TOTAL=$(wc -l <"$DISCOVERY_FILE" | tr -d ' ')

if [[ "$TOTAL" -eq 0 ]]; then
  echo "FAIL: no silent-skip integration tests discovered" >&2
  exit 2
fi

if [[ "$MODE" == "discover" ]]; then
  cat "$DISCOVERY_FILE"
  echo
  echo "discovered $TOTAL test targets" >&2
  if [[ "$TOTAL" -lt 40 ]]; then
    echo "WARN: only $TOTAL targets (expected ≥40); see runbook" >&2
  fi
  exit 0
fi

# ---- run ----------------------------------------------------------
mkdir -p "$REPORT_DIR"
TS=$(date -u +%Y%m%dT%H%M%SZ)
REPORT_PATH="$REPORT_DIR/silent-skip-report-$TS.json"
TMP_DIR="$TMP_PARENT/run"
mkdir -p "$TMP_DIR"

# Locate `timeout`. macOS via `brew install coreutils` → `gtimeout`.
TIMEOUT_BIN=""
if command -v timeout >/dev/null 2>&1; then
  TIMEOUT_BIN="timeout"
elif command -v gtimeout >/dev/null 2>&1; then
  TIMEOUT_BIN="gtimeout"
else
  echo "WARN: no timeout/gtimeout; per-target timeout disabled" >&2
fi

START_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ)
PASSED=0; FAILED=0; TIMEOUT=0; COMPLETED=0
FAILURES_JSON=""

log() { echo "[$(date -u +%H:%M:%S)] $*" >&2; }

json_tail() {
  # tail-of-stderr JSON-encoder (handles quotes, newlines). Falls
  # back to bare `""` if python3 absent.
  if command -v python3 >/dev/null 2>&1; then
    tail -8 "$1" | python3 -c \
      'import json,sys; print(json.dumps(sys.stdin.read()))'
  else
    echo '""'
  fi
}

mode_label="nightly"
[[ "$LOCAL" -eq 1 ]] && mode_label="local"
log "silent-skip $mode_label — $TOTAL targets, ${TIMEOUT_SECS}s per-target"

while IFS= read -r entry; do
  read -r crate target _ <<<"$entry"
  log "→ $crate :: $target"
  out_file="$TMP_DIR/$crate-$target.log"
  if [[ -n "$TIMEOUT_BIN" ]]; then
    "$TIMEOUT_BIN" --kill-after=10s "$TIMEOUT_SECS" \
      cargo test --quiet -p "$crate" --test "$target" \
      >"$out_file" 2>&1
  else
    cargo test --quiet -p "$crate" --test "$target" >"$out_file" 2>&1
  fi
  rc=$?
  COMPLETED=$(( COMPLETED + 1 ))
  case "$rc" in
    0)        PASSED=$(( PASSED + 1 )) ;;
    124|137)
      TIMEOUT=$(( TIMEOUT + 1 ))
      tail_json=$(json_tail "$out_file")
      FAILURES_JSON+="    { \"test\": \"$target\", \"crate\": \"$crate\","
      FAILURES_JSON+=" \"exit_code\": $rc, \"outcome\": \"timeout\","
      FAILURES_JSON+=" \"stderr_tail\": $tail_json },
"
      ;;
    *)
      FAILED=$(( FAILED + 1 ))
      tail_json=$(json_tail "$out_file")
      FAILURES_JSON+="    { \"test\": \"$target\", \"crate\": \"$crate\","
      FAILURES_JSON+=" \"exit_code\": $rc, \"outcome\": \"failed\","
      FAILURES_JSON+=" \"stderr_tail\": $tail_json },
"
      ;;
  esac
done <"$DISCOVERY_FILE"

END_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ)
# Silent-skip exits 0; cargo counts them as `passed`. See runbook
# for the future `#[ignore]`-attribute migration plan.
SKIPPED=0
FAILURES_JSON="${FAILURES_JSON%,
}"

cat >"$REPORT_PATH" <<JSON
{
  "started": "$START_ISO",
  "completed_at": "$END_ISO",
  "mode": "$mode_label",
  "total": $TOTAL,
  "completed": $COMPLETED,
  "passed": $PASSED,
  "failed": $FAILED,
  "timeout": $TIMEOUT,
  "skipped": $SKIPPED,
  "failures": [
$FAILURES_JSON
  ]
}
JSON

{
  echo
  echo "=== silent-skip nightly summary ==="
  echo "  mode:       $mode_label"
  echo "  total:      $TOTAL"
  echo "  passed:     $PASSED"
  echo "  failed:     $FAILED"
  echo "  timeout:    $TIMEOUT"
  echo "  report:     $REPORT_PATH"
} >&2

exit 0
