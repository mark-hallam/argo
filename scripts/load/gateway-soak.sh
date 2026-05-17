#!/usr/bin/env bash
# Whole-system gateway load + soak harness. Drives the running argo
# gateway with one of three profiles (burst / sustained / spike) or a
# multi-minute soak, captures p50/p95/p99 latency, RSS/CPU growth,
# SQLite store + audit-row delta, and writes a single JSON results
# blob to ${REPORT_DIR}/skycot-load-soak-<ts>-<profile>.json.
#
# Pure-external: we drive the gateway over HTTP and sample `ps` / file
# size. No gateway code is touched. See scripts/load/README.md.
#
# See brief: docs/build-plan/streams/codex-gateway-load-soak-baseline.md
# Mission alignment: reliability (CLAUDE.md #1) + observability (#2).
#
# Sensible deviations from the brief:
#   * Brief cites `/healthz` and `/meta/tools`. The actual gateway
#     surface uses `/health` and does not expose `/meta/tools`. We
#     drive `/health` + `/version` + `/routes` (all in the public-GET
#     allowlist) which preserves the brief's intent: cheap, read-only
#     endpoints under no auth.
#   * Spike ramp is approximated as 5 × 2s buckets (100/200/300/400/500
#     RPS) + 30s hold @ 500 RPS, since `oha` cannot ramp a single run.

set -uo pipefail

# ---- config / thresholds ------------------------------------------
# Edit here to retune pass/warn/fail bands. All thresholds documented
# in scripts/load/README.md → "Interpreting output".
P95_OK_MS=500
P95_WARN_MS=2000     # warn band: 500..=2000ms; >2000ms = FAIL
RSS_GROWTH_OK_PCT=10
RSS_GROWTH_WARN_PCT=50   # >50% growth in a 5-min sustained = memleak
DB_BYTES_PER_ROW_OK=2048 # >2KB per audit row = degenerate growth

DEFAULT_GATEWAY_PORT=7400
DEFAULT_REPORT_DIR="${TMPDIR:-/tmp}"

# ---- argv ---------------------------------------------------------
usage() {
  cat <<'USAGE'
Usage: scripts/load/gateway-soak.sh [options]

Drives a local or remote argo gateway with one load profile and
records a single JSON results file.

Options:
  --profile <burst|sustained|spike>  Which profile to run. Required
                                     unless --soak is given.
  --soak <minutes>                   Run the sustained profile for N
                                     minutes (overrides --profile).
                                     Default off.
  --target-url <url>                 Drive a remote gateway instead
                                     of spawning one locally.
                                     Format: http://host:port .
  --port <port>                      Local gateway port (default 7400).
  --bind <addr>                      Local gateway bind (default
                                     127.0.0.1).
  --report-dir <path>                Where to drop the JSON results
                                     file (default $TMPDIR or /tmp).
  --argo-bin <path>                  Path to argo binary (default
                                     `argo` on PATH).
  --keep-data                        Don't wipe the temp ARGO_HOME on
                                     exit. Useful for inspecting the
                                     resulting db.sqlite.
  -h, --help                         This text.

Profiles:
  burst      100 RPS → /health for 60s.
  sustained  50 RPS mix of /health + /routes for 5 minutes.
  spike      0→500 RPS ramp in 10s (5 buckets × 2s), hold 30s, drop.

Examples:
  scripts/load/gateway-soak.sh --profile burst
  scripts/load/gateway-soak.sh --soak 30
  scripts/load/gateway-soak.sh --profile sustained --target-url \
    http://staging.skycot.internal:7400

Exit codes:
  0   profile completed; thresholds OK (see results JSON).
  1   profile completed; one or more thresholds in WARN/FAIL band.
  2   setup error (missing oha, gateway didn't come up, bad args).
USAGE
}

PROFILE=""
SOAK_MIN=""
TARGET_URL=""
GATEWAY_PORT="$DEFAULT_GATEWAY_PORT"
GATEWAY_BIND="127.0.0.1"
REPORT_DIR="$DEFAULT_REPORT_DIR"
ARGO_BIN="argo"
KEEP_DATA=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) PROFILE="${2:?--profile requires an argument}"; shift 2;;
    --soak)    SOAK_MIN="${2:?--soak requires minutes}"; shift 2;;
    --target-url) TARGET_URL="${2:?--target-url requires a URL}"; shift 2;;
    --port)    GATEWAY_PORT="${2:?--port requires a number}"; shift 2;;
    --bind)    GATEWAY_BIND="${2:?--bind requires an address}"; shift 2;;
    --report-dir) REPORT_DIR="${2:?--report-dir requires a path}"; shift 2;;
    --argo-bin) ARGO_BIN="${2:?--argo-bin requires a path}"; shift 2;;
    --keep-data) KEEP_DATA=1; shift;;
    -h|--help) usage; exit 0;;
    *) echo "ERROR: unknown arg: $1" >&2; usage >&2; exit 2;;
  esac
done

if [[ -n "$SOAK_MIN" ]]; then
  PROFILE="sustained"
elif [[ -z "$PROFILE" ]]; then
  echo "ERROR: must pass --profile <burst|sustained|spike> or --soak <min>" >&2
  exit 2
fi

case "$PROFILE" in
  burst|sustained|spike) ;;
  *) echo "ERROR: unknown profile: $PROFILE" >&2; exit 2;;
esac

if ! command -v oha >/dev/null 2>&1; then
  cat <<'NEED_OHA' >&2
ERROR: `oha` not on PATH. Install with one of:
  brew install oha            # macOS
  cargo install oha           # any OS with cargo
See scripts/load/README.md → "Prerequisites".
NEED_OHA
  exit 2
fi

mkdir -p "$REPORT_DIR"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
AGGREGATOR="$SCRIPT_DIR/aggregate-results.py"
if [[ ! -x "$AGGREGATOR" && ! -r "$AGGREGATOR" ]]; then
  echo "ERROR: missing aggregator at $AGGREGATOR" >&2
  exit 2
fi

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
RESULTS_PATH="$REPORT_DIR/skycot-load-soak-${TIMESTAMP}-${PROFILE}.json"
WORKDIR="$(mktemp -d -t skycot-load-soak.XXXXXX)"
RUN_DIR="$WORKDIR/run"
mkdir -p "$RUN_DIR"
SAMPLES_FILE="$RUN_DIR/samples.tsv"
: >"$SAMPLES_FILE"

# ---- portable file helpers (linux + macOS) -----------------------
file_size_bytes() {
  local f="$1"
  [[ -f "$f" ]] || { echo 0; return; }
  if stat -c %s "$f" 2>/dev/null; then return; fi
  stat -f %z "$f" 2>/dev/null || wc -c <"$f" | tr -d ' '
}

audit_row_count() {
  local db="$1"
  [[ -f "$db" ]] || { echo 0; return; }
  command -v sqlite3 >/dev/null 2>&1 || { echo 0; return; }
  # `audit_log` is the canonical chain table; tolerate either name.
  sqlite3 "$db" \
    "SELECT COALESCE((SELECT COUNT(*) FROM audit_log), \
                     (SELECT COUNT(*) FROM audit), 0);" \
    2>/dev/null || echo 0
}

# ---- gateway lifecycle -------------------------------------------
GATEWAY_PID=""
SAMPLER_PID=""
HOME_DIR=""

log() { echo "[$(date -u +%H:%M:%S)] $*" >&2; }

# shellcheck disable=SC2329  # invoked via `trap` below, not directly.
cleanup() {
  local rc=$?
  [[ -n "$SAMPLER_PID" ]] && kill "$SAMPLER_PID" 2>/dev/null || true
  [[ -n "$GATEWAY_PID" ]] && kill "$GATEWAY_PID" 2>/dev/null || true
  if [[ -z "$TARGET_URL" && "$KEEP_DATA" -eq 0 ]]; then
    rm -rf "$WORKDIR" "$HOME_DIR" 2>/dev/null || true
  else
    log "kept WORKDIR=$WORKDIR HOME_DIR=$HOME_DIR"
  fi
  exit "$rc"
}
trap cleanup EXIT INT TERM

if [[ -n "$TARGET_URL" ]]; then
  BASE_URL="${TARGET_URL%/}"
  log "target mode — driving $BASE_URL (no local gateway spawn)"
else
  command -v "$ARGO_BIN" >/dev/null 2>&1 || {
    echo "ERROR: argo binary '$ARGO_BIN' not on PATH" >&2; exit 2; }
  HOME_DIR="$(mktemp -d -t skycot-load-home.XXXXXX)"
  export ARGO_HOME="$HOME_DIR"
  log "spawning local gateway (ARGO_HOME=$HOME_DIR, port=$GATEWAY_PORT)"
  "$ARGO_BIN" workspace init >"$RUN_DIR/workspace-init.log" 2>&1 || {
    echo "ERROR: workspace init failed; see $RUN_DIR/workspace-init.log" >&2
    exit 2; }
  "$ARGO_BIN" gateway start --bind "$GATEWAY_BIND" --port "$GATEWAY_PORT" \
    --quiet >"$RUN_DIR/gateway.log" 2>&1 &
  GATEWAY_PID=$!
  BASE_URL="http://${GATEWAY_BIND}:${GATEWAY_PORT}"
  for _ in $(seq 1 60); do
    if curl -fsS "$BASE_URL/health" >/dev/null 2>&1; then break; fi
    sleep 0.5
  done
  if ! curl -fsS "$BASE_URL/health" >/dev/null 2>&1; then
    echo "ERROR: gateway never became healthy at $BASE_URL" >&2
    tail -50 "$RUN_DIR/gateway.log" >&2 || true
    exit 2
  fi
  log "gateway healthy (pid=$GATEWAY_PID)"
fi

# ---- 1Hz ps sampler (local mode only) ----------------------------
# Each line: "<unix_ts>\t<rss_kb>\t<cpu_pct>". `ps -o` returns rss in
# KB and %cpu as the lifetime average since process start on macOS,
# instantaneous on Linux — both acceptable for trend detection.
if [[ -n "$GATEWAY_PID" ]]; then
  (
    while kill -0 "$GATEWAY_PID" 2>/dev/null; do
      ts="$(date +%s)"
      line="$(ps -o rss=,%cpu= -p "$GATEWAY_PID" 2>/dev/null \
              | awk -v t="$ts" '{ printf "%d\t%d\t%s\n", t, $1, $2 }')"
      [[ -n "$line" ]] && echo "$line" >>"$SAMPLES_FILE"
      sleep 1
    done
  ) &
  SAMPLER_PID=$!
fi

# ---- baseline snapshots ------------------------------------------
DB_PATH=""
if [[ -n "$HOME_DIR" ]]; then
  # Argo writes the SQLite store to $ARGO_HOME/store.sqlite by default
  # but older builds used db.sqlite. Pick whichever exists post-init.
  for cand in "$HOME_DIR/store.sqlite" "$HOME_DIR/db.sqlite"; do
    [[ -f "$cand" ]] && { DB_PATH="$cand"; break; }
  done
fi
DB_BYTES_START="$(file_size_bytes "$DB_PATH")"
AUDIT_ROWS_START="$(audit_row_count "$DB_PATH")"

# ---- profile runner ----------------------------------------------
# Run one oha invocation. Each invocation writes its raw JSON to
# $RUN_DIR/<tag>.json so the aggregator can roll them up.
run_oha() {
  local tag="$1" url="$2" rate="$3" duration_s="$4"
  local out="$RUN_DIR/${tag}.json"
  local connections=$(( rate < 50 ? 50 : (rate > 200 ? 200 : rate) ))
  oha -z "${duration_s}s" -q "$rate" -c "$connections" \
      --no-tui -j "$url" >"$out" 2>"$RUN_DIR/${tag}.err" || {
    log "WARN: oha for $tag exited non-zero — see ${tag}.err"
  }
}

START_EPOCH=$(date +%s)
START_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ)
TAGS=()

run_burst() {
  log "profile=burst — 100 RPS → /health for 60s"
  run_oha burst_health "$BASE_URL/health" 100 60
  TAGS+=("burst_health")
}

run_sustained() {
  local duration_s="${1:-300}"
  log "profile=sustained — 50 RPS mix /health+/routes for ${duration_s}s"
  # 25 RPS each, in parallel, approximates a 50 RPS mix.
  run_oha sustained_health "$BASE_URL/health" 25 "$duration_s" &
  local p1=$!
  run_oha sustained_routes "$BASE_URL/routes" 25 "$duration_s" &
  local p2=$!
  wait "$p1" "$p2"
  TAGS+=("sustained_health" "sustained_routes")
}

run_spike() {
  log "profile=spike — ramp 0→500 RPS in 10s, hold 30s @ 500"
  local i rate
  for i in 1 2 3 4 5; do
    rate=$(( i * 100 ))
    run_oha "spike_ramp_${i}" "$BASE_URL/health" "$rate" 2
    TAGS+=("spike_ramp_${i}")
  done
  run_oha "spike_hold" "$BASE_URL/health" 500 30
  TAGS+=("spike_hold")
}

if [[ -n "$SOAK_MIN" ]]; then
  duration_s=$(( SOAK_MIN * 60 ))
  log "soak mode — running sustained for ${SOAK_MIN}min (${duration_s}s)"
  run_sustained "$duration_s"
else
  case "$PROFILE" in
    burst)     run_burst ;;
    sustained) run_sustained 300 ;;
    spike)     run_spike ;;
  esac
fi

# ---- post-run snapshots ------------------------------------------
# Stop the sampler so the final tally is stable.
if [[ -n "$SAMPLER_PID" ]]; then
  kill "$SAMPLER_PID" 2>/dev/null || true
  wait "$SAMPLER_PID" 2>/dev/null || true
  SAMPLER_PID=""
fi

END_EPOCH=$(date +%s)
END_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ)
DB_BYTES_END="$(file_size_bytes "$DB_PATH")"
AUDIT_ROWS_END="$(audit_row_count "$DB_PATH")"

# ---- aggregate ---------------------------------------------------
python3 "$AGGREGATOR" \
  --profile "$PROFILE" \
  --soak-min "${SOAK_MIN:-0}" \
  --start "$START_ISO" \
  --end "$END_ISO" \
  --duration-s "$(( END_EPOCH - START_EPOCH ))" \
  --base-url "$BASE_URL" \
  --samples "$SAMPLES_FILE" \
  --db-bytes-start "$DB_BYTES_START" \
  --db-bytes-end "$DB_BYTES_END" \
  --audit-rows-start "$AUDIT_ROWS_START" \
  --audit-rows-end "$AUDIT_ROWS_END" \
  --p95-ok-ms "$P95_OK_MS" \
  --p95-warn-ms "$P95_WARN_MS" \
  --rss-growth-ok-pct "$RSS_GROWTH_OK_PCT" \
  --rss-growth-warn-pct "$RSS_GROWTH_WARN_PCT" \
  --db-bytes-per-row-ok "$DB_BYTES_PER_ROW_OK" \
  --run-dir "$RUN_DIR" \
  --tags "${TAGS[*]}" \
  --out "$RESULTS_PATH"

AGG_RC=$?

log "results → $RESULTS_PATH"
log "verdict → $(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["verdict"])' "$RESULTS_PATH" 2>/dev/null || echo unknown)"

exit "$AGG_RC"
