#!/usr/bin/env bash
# SKYCOT 4-hour multi-worker soak test. Spins 1 gateway + N workers,
# drives a steady prompt stream for ~4h (or 20min in fast-mode), and
# asserts five load-bearing invariants: no-crash, memory growth ≤ 1.5×,
# fh growth ≤ 1.5×, audit chain intact, gateway-200. Writes a JSON
# summary to target/soak-report-<ts>.json. Linux-only by design.
# See docs/skycot/runbook/soak-test.md.

set -uo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/soak-skycot.sh [argo-binary]

Multi-worker soak. Default cadence: 4 workers × 5 prompts/min for 240 min.

Env vars:
  ARGO_SOAK_WORKERS       worker count                   (default 4)
  ARGO_SOAK_PROVIDER      provider id                    (default stub-echo)
  ARGO_SOAK_VERTICAL      vertical id                    (default gtm-engineer)
  ARGO_SOAK_GATEWAY_PORT  gateway TCP port               (default 7400)
  ARGO_SOAK_PROMPTS_PER_WORKER_MIN  per-worker rate      (default 5)
  ARGO_SOAK_FAST          1 = 20-min mode (CI/dev-loop)
  ARGO_SOAK_DURATION_OVERRIDE_MIN  hard duration override (min);
                          wins over FAST; used by the test harness.
  ARGO_SOAK_REPORT_DIR    JSON summary dir               (default ./target)

Exits: 0 = all 5 assertions pass; 1 = one or more failed; 2 = setup error.
USAGE
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage; exit 0
fi

# ---- config -------------------------------------------------------
ARGO_BIN="${1:-argo}"
WORKERS="${ARGO_SOAK_WORKERS:-4}"
PROVIDER="${ARGO_SOAK_PROVIDER:-stub-echo}"
VERTICAL="${ARGO_SOAK_VERTICAL:-gtm-engineer}"
GATEWAY_PORT="${ARGO_SOAK_GATEWAY_PORT:-7400}"
PROMPTS_PER_WORKER_MIN="${ARGO_SOAK_PROMPTS_PER_WORKER_MIN:-5}"
REPORT_DIR="${ARGO_SOAK_REPORT_DIR:-$(pwd)/target}"

# Duration resolution: override → fast → default 4h.
if [[ -n "${ARGO_SOAK_DURATION_OVERRIDE_MIN:-}" ]]; then
  DURATION_MIN="$ARGO_SOAK_DURATION_OVERRIDE_MIN"
  SNAPSHOT_INTERVAL_S=10
  BASELINE_OFFSET_S=20
elif [[ -n "${ARGO_SOAK_FAST:-}" && "${ARGO_SOAK_FAST}" != "0" ]]; then
  DURATION_MIN=20
  SNAPSHOT_INTERVAL_S=60
  BASELINE_OFFSET_S=120
else
  DURATION_MIN=240
  SNAPSHOT_INTERVAL_S=300
  BASELINE_OFFSET_S=1800
fi
DURATION_S=$(( DURATION_MIN * 60 ))

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH" >&2
  exit 2
fi

WORKDIR="$(mktemp -d)"
HOME_DIR="$(mktemp -d)"
LOG_DIR="$WORKDIR/logs"
mkdir -p "$LOG_DIR" "$REPORT_DIR"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export ARGO_HOME="$HOME_DIR"
export ARGO_VERTICALS_DIR="$SCRIPT_DIR/../verticals"

START_EPOCH=$(date +%s)
START_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ)
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
REPORT_PATH="$REPORT_DIR/soak-report-$TIMESTAMP.json"

PIDS=()
GATEWAY_PID=""
PROMPTS_SENT=0
PROMPTS_SUCCEEDED=0

cleanup() {
  local rc=$?
  for pid in "${PIDS[@]}" "$GATEWAY_PID"; do
    [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
  done
  rm -rf "$WORKDIR"
  exit "$rc"
}
trap cleanup EXIT INT TERM

log() { echo "[$(date -u +%H:%M:%S)] $*" >&2; }

# ---- bring up workspace + gateway + workers -----------------------
log "soak start — duration=${DURATION_MIN}min workers=${WORKERS} provider=${PROVIDER} vertical=${VERTICAL}"
cd "$WORKDIR" || exit 2
"$ARGO_BIN" workspace init >/dev/null 2>&1 || {
  echo "FAIL: workspace init" >&2; exit 2; }

"$ARGO_BIN" gateway start --bind 127.0.0.1 --port "$GATEWAY_PORT" --quiet \
  >"$LOG_DIR/gateway.log" 2>&1 &
GATEWAY_PID=$!
# Wait for /health to respond (≤ 15s).
for _ in $(seq 1 30); do
  if curl -fsS "http://127.0.0.1:${GATEWAY_PORT}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done
if ! curl -fsS "http://127.0.0.1:${GATEWAY_PORT}/health" >/dev/null 2>&1; then
  echo "FAIL: gateway never became healthy on port ${GATEWAY_PORT}" >&2
  cat "$LOG_DIR/gateway.log" >&2 || true
  exit 2
fi
log "gateway up (pid=$GATEWAY_PID port=$GATEWAY_PORT)"

for i in $(seq 1 "$WORKERS"); do
  "$ARGO_BIN" work serve --vertical "$VERTICAL" --interval-seconds 1 \
    --provider mock --model mock --max-steps 1 \
    >"$LOG_DIR/worker-$i.log" 2>&1 &
  PIDS+=($!)
done
log "spawned ${#PIDS[@]} workers: ${PIDS[*]}"

# ---- snapshot helpers --------------------------------------------
fh_count() {
  # Prefer /proc fd count (cheap); fall back to lsof.
  local pid="$1"
  if [[ -d "/proc/$pid/fd" ]]; then
    # shellcheck disable=SC2012
    ls "/proc/$pid/fd" 2>/dev/null | wc -l | tr -d ' '
  else
    lsof -p "$pid" 2>/dev/null | wc -l | tr -d ' '
  fi
}

rss_kb() {
  local pid="$1"
  ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ' || echo 0
}

# Output: "rss_total_kb fh_total"
snapshot() {
  local rss_total=0 fh_total=0 r f
  for pid in "${PIDS[@]}" "$GATEWAY_PID"; do
    if kill -0 "$pid" 2>/dev/null; then
      r=$(rss_kb "$pid"); f=$(fh_count "$pid")
      rss_total=$(( rss_total + ${r:-0} ))
      fh_total=$(( fh_total + ${f:-0} ))
    fi
  done
  echo "$rss_total $fh_total"
}

# ---- producer + snapshot loop -------------------------------------
# Submission cadence: 4 workers × 5 prompts/min = 20 prompts/min.
TOTAL_RATE=$(( WORKERS * PROMPTS_PER_WORKER_MIN ))
SUBMIT_INTERVAL_S=$(awk -v r="$TOTAL_RATE" 'BEGIN { printf "%.3f", 60.0 / r }')
BASELINE_RSS=0
BASELINE_FH=0
NOW=$(date +%s)
NEXT_SNAPSHOT=$(( NOW + SNAPSHOT_INTERVAL_S ))
BASELINE_AT=$(( NOW + BASELINE_OFFSET_S ))
END_AT=$(( NOW + DURATION_S ))

while (( $(date +%s) < END_AT )); do
  add_out=$("$ARGO_BIN" work add --vertical "$VERTICAL" \
    --prompt "what is 2+2" --json 2>>"$LOG_DIR/producer.log")
  PROMPTS_SENT=$(( PROMPTS_SENT + 1 ))
  if [[ -n "$add_out" ]] && echo "$add_out" | grep -q '"job_id"'; then
    PROMPTS_SUCCEEDED=$(( PROMPTS_SUCCEEDED + 1 ))
  fi

  NOW=$(date +%s)
  if (( NOW >= BASELINE_AT && BASELINE_RSS == 0 )); then
    read -r BASELINE_RSS BASELINE_FH < <(snapshot)
    log "baseline snapshot: rss=${BASELINE_RSS}kb fh=${BASELINE_FH}"
  fi
  if (( NOW >= NEXT_SNAPSHOT )); then
    read -r r f < <(snapshot)
    elapsed_min=$(( (NOW - START_EPOCH) / 60 ))
    log "t=${elapsed_min}min rss=${r}kb fh=${f} sent=${PROMPTS_SENT}"
    NEXT_SNAPSHOT=$(( NOW + SNAPSHOT_INTERVAL_S ))
  fi
  sleep "$SUBMIT_INTERVAL_S"
done

# ---- final snapshot + assertions ----------------------------------
read -r FINAL_RSS FINAL_FH < <(snapshot)
# If we never crossed the baseline window (very short test), use the
# final snapshot as both baseline and final — growth is 1.0× trivially.
if (( BASELINE_RSS == 0 )); then
  BASELINE_RSS="$FINAL_RSS"; BASELINE_FH="$FINAL_FH"
fi
[[ "$BASELINE_RSS" -le 0 ]] && BASELINE_RSS=1
[[ "$BASELINE_FH" -le 0 ]] && BASELINE_FH=1

MEM_PCT=$(awk -v a="$FINAL_RSS" -v b="$BASELINE_RSS" \
  'BEGIN { printf "%.2f", (a/b - 1.0) * 100.0 }')
FH_PCT=$(awk -v a="$FINAL_FH" -v b="$BASELINE_FH" \
  'BEGIN { printf "%.2f", (a/b - 1.0) * 100.0 }')

# 1. no-crash
CRASHED=0
for pid in "${PIDS[@]}"; do
  kill -0 "$pid" 2>/dev/null || CRASHED=$(( CRASHED + 1 ))
done
A_NO_CRASH="true"; (( CRASHED > 0 )) && A_NO_CRASH="false"

# 2 + 3. growth
A_MEM=$(awk -v p="$MEM_PCT" 'BEGIN { print (p <= 50.0) ? "true" : "false" }')
A_FH=$(awk  -v p="$FH_PCT"  'BEGIN { print (p <= 50.0) ? "true" : "false" }')

# 4. audit chain
if "$ARGO_BIN" audit verify --json >"$LOG_DIR/audit-verify.json" 2>&1; then
  A_AUDIT=$(grep -q '"ok":true' "$LOG_DIR/audit-verify.json" && echo true || echo false)
else
  A_AUDIT="false"
fi

# 5. gateway still serving
A_GATEWAY=$(curl -fsS "http://127.0.0.1:${GATEWAY_PORT}/health" >/dev/null 2>&1 \
  && echo true || echo false)

ALL_PASS=$([[ "$A_NO_CRASH" == "true" && "$A_MEM" == "true" && "$A_FH" == "true" \
  && "$A_AUDIT" == "true" && "$A_GATEWAY" == "true" ]] && echo true || echo false)

END_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ)
cat >"$REPORT_PATH" <<JSON
{
  "start": "$START_ISO",
  "end": "$END_ISO",
  "duration_min": $DURATION_MIN,
  "workers": $WORKERS,
  "provider": "$PROVIDER",
  "vertical": "$VERTICAL",
  "prompts_sent": $PROMPTS_SENT,
  "prompts_succeeded": $PROMPTS_SUCCEEDED,
  "baseline_rss_kb": $BASELINE_RSS,
  "final_rss_kb": $FINAL_RSS,
  "memory_growth_pct": $MEM_PCT,
  "baseline_fh": $BASELINE_FH,
  "final_fh": $FINAL_FH,
  "fh_growth_pct": $FH_PCT,
  "audit_chain_intact": $A_AUDIT,
  "assertions": {
    "no_crash": $A_NO_CRASH,
    "memory_growth_ok": $A_MEM,
    "fh_growth_ok": $A_FH,
    "audit_chain_intact": $A_AUDIT,
    "gateway_healthy": $A_GATEWAY,
    "all_pass": $ALL_PASS
  }
}
JSON

{
  echo
  echo "=== soak summary ==="
  echo "  duration:        ${DURATION_MIN} min"
  echo "  prompts:         ${PROMPTS_SUCCEEDED} / ${PROMPTS_SENT}"
  echo "  rss baseline:    ${BASELINE_RSS} kb"
  echo "  rss final:       ${FINAL_RSS} kb   (${MEM_PCT}% growth)"
  echo "  fh baseline:     ${BASELINE_FH}"
  echo "  fh final:        ${FINAL_FH}   (${FH_PCT}% growth)"
  echo "  no_crash:        ${A_NO_CRASH}"
  echo "  memory_ok:       ${A_MEM}"
  echo "  fh_ok:           ${A_FH}"
  echo "  audit_intact:    ${A_AUDIT}"
  echo "  gateway_healthy: ${A_GATEWAY}"
  echo "  all_pass:        ${ALL_PASS}"
  echo "  report:          ${REPORT_PATH}"
} >&2

[[ "$ALL_PASS" == "true" ]] && exit 0 || exit 1
