#!/usr/bin/env bash
# check-worker-heartbeats.sh — conductor-side detection of wedged workers.
#
# Workers update /tmp/argo-worker-pulse/<brief-id> every 30s during their
# iteration (see claude-worker-loop.sh). If the iteration wedges (hung
# cargo, network partition, machine sleep), the pulse file's mtime goes
# stale.
#
# This script reports pulses that haven't been touched in >5 minutes —
# strong signal of a wedged worker. The conductor can then either:
#   - Force-release the claim back to TODO (use scripts/force-release.sh)
#   - Investigate via the worker log at ~/.argo/worker-logs/*-<brief-id>.ndjson
#   - Wait if the worker is on a known long-running cargo command
#
# Pulse age threshold (5min) is much tighter than the af500037
# >2h-since-push proxy. Heartbeat is the direct signal; >2h is downstream.
#
# Usage:
#   scripts/check-worker-heartbeats.sh                # report stale pulses
#   scripts/check-worker-heartbeats.sh --threshold 600 # custom threshold (sec)
#   scripts/check-worker-heartbeats.sh --json
#
# Exit codes:
#   0 — no stale pulses
#   1 — one or more pulses stale (worker(s) likely wedged)
#   2 — invocation error

set -uo pipefail

PULSE_DIR="/tmp/argo-worker-pulse"
THRESHOLD_SEC=300  # 5 min
MODE="text"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --threshold) THRESHOLD_SEC="${2:-300}"; shift 2 ;;
        --threshold=*) THRESHOLD_SEC="${1#--threshold=}"; shift ;;
        --json) MODE="json"; shift ;;
        --help|-h) sed -n '2,21p' "$0" | sed 's/^# \?//'; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [[ ! -d "$PULSE_DIR" ]]; then
    if [[ "$MODE" == "json" ]]; then
        echo '{"pulses_total":0,"pulses_stale":0,"stale":[]}'
    else
        echo "No pulse directory at $PULSE_DIR — no workers running."
    fi
    exit 0
fi

NOW_EPOCH=$(date +%s)
TOTAL=0
STALE_COUNT=0
STALE_LINES=""

for f in "$PULSE_DIR"/*; do
    [[ ! -e "$f" ]] && continue  # empty directory glob fallthrough
    TOTAL=$((TOTAL + 1))
    brief=$(basename "$f")
    # mtime via stat; works on BSD (macOS) and GNU
    mtime=$(stat -f %m "$f" 2>/dev/null || stat -c %Y "$f" 2>/dev/null || echo 0)
    age=$((NOW_EPOCH - mtime))
    if [[ "$age" -gt "$THRESHOLD_SEC" ]]; then
        STALE_COUNT=$((STALE_COUNT + 1))
        age_min=$((age / 60))
        STALE_LINES="${STALE_LINES}  ${age_min}min  $brief\n"
    fi
done

case "$MODE" in
    json)
        # Build stale array
        stale_arr=""
        if [[ "$STALE_COUNT" -gt 0 ]]; then
            stale_arr=$(echo -e "$STALE_LINES" | awk 'NF { gsub(/^ +| +$/, ""); printf "{\"age_min\":%d,\"brief\":\"%s\"}", $1, $2; if (NR < c) printf "," }' c="$STALE_COUNT")
        fi
        printf '{"pulses_total":%d,"pulses_stale":%d,"stale":[%s]}\n' \
            "$TOTAL" "$STALE_COUNT" "$stale_arr"
        ;;
    text|*)
        echo "=== Worker heartbeat check ==="
        echo "Pulses tracked: $TOTAL"
        echo "Stale (>${THRESHOLD_SEC}s): $STALE_COUNT"
        if [[ "$STALE_COUNT" -gt 0 ]]; then
            echo ""
            echo "Likely-wedged workers:"
            printf "$STALE_LINES"
            echo ""
            echo "Investigate via: tail -50 ~/.argo/worker-logs/*-<brief-id>.ndjson"
            echo "Force-release: edit BUILD_STATUS row CLAIMED→TODO with a salvage note."
        fi
        ;;
esac

[[ "$STALE_COUNT" -gt 0 ]] && exit 1 || exit 0
