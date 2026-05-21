#!/usr/bin/env bash
# check-agent-heartbeats.sh — agent liveness watchdog
#
# Reads every heartbeat JSON under docs/skycot/agents/heartbeats/ and
# reports whether each agent is alive (last run within expected cadence
# + 25% grace) or stale.
#
# Used by:
#   - operator on demand: `bash scripts/check-agent-heartbeats.sh`
#   - conductor's hourly tick Step 4.6 (auto-surfaces stale agents)
#
# Exit code:
#   0 — all agents alive (or only marketing stale on a weekend, which is expected)
#   1 — at least one agent stale; details to stderr

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HEARTBEAT_DIR="$ROOT/docs/skycot/agents/heartbeats"

if [[ ! -d "$HEARTBEAT_DIR" ]]; then
    echo "no heartbeat dir at $HEARTBEAT_DIR — agents may not have started yet" >&2
    exit 1
fi

now_epoch="$(date -u +%s)"
now_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
day_of_week_local="$(date +%u)"  # 1=Monday, 7=Sunday in operator's local tz

stale_count=0
alive_count=0

printf '%-15s  %-22s  %-12s  %s\n' 'AGENT' 'LAST RUN (UTC)' 'AGE' 'STATUS'
printf '%-15s  %-22s  %-12s  %s\n' '─────' '─────────────────────' '────────────' '──────'

shopt -s nullglob
for hb in "$HEARTBEAT_DIR"/*.json; do
    agent="$(basename "$hb" .json)"
    [[ "$agent" == "README" ]] && continue

    # Parse JSON fields. Use python rather than jq to keep dep light.
    last_iso="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('last_run_utc',''))" "$hb")"
    cadence_s="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('expected_cadence_seconds',0))" "$hb")"

    if [[ -z "$last_iso" || -z "$cadence_s" || "$cadence_s" == "0" ]]; then
        printf '%-15s  %-22s  %-12s  ⚠️  MALFORMED HEARTBEAT\n' "$agent" "(unparseable)" "?"
        stale_count=$((stale_count + 1))
        continue
    fi

    # Convert ISO to epoch (BSD date on macOS + GNU date on Linux both supported).
    if last_epoch="$(date -u -j -f '%Y-%m-%dT%H:%M:%SZ' "$last_iso" +%s 2>/dev/null)" \
        || last_epoch="$(date -u -d "$last_iso" +%s 2>/dev/null)"; then
        :
    else
        printf '%-15s  %-22s  %-12s  ⚠️  TIMESTAMP UNPARSEABLE\n' "$agent" "$last_iso" "?"
        stale_count=$((stale_count + 1))
        continue
    fi

    age_s=$((now_epoch - last_epoch))
    grace_s=$((cadence_s * 5 / 4))  # cadence × 1.25

    # Format age human-readably
    if (( age_s < 60 )); then
        age_h="${age_s}s"
    elif (( age_s < 3600 )); then
        age_h="$((age_s / 60))m"
    elif (( age_s < 86400 )); then
        age_h="$((age_s / 3600))h$((age_s % 3600 / 60))m"
    else
        age_h="$((age_s / 86400))d$((age_s % 86400 / 3600))h"
    fi

    # Special-case: marketing agent silence on weekends (Sat=6, Sun=7 in %u) is expected.
    if [[ "$agent" == "marketing" && ( "$day_of_week_local" == "6" || "$day_of_week_local" == "7" ) ]]; then
        if (( age_s > grace_s )); then
            printf '%-15s  %-22s  %-12s  ☀️  WEEKEND (silence expected)\n' "$agent" "$last_iso" "$age_h"
            alive_count=$((alive_count + 1))
            continue
        fi
    fi

    if (( age_s > grace_s )); then
        printf '%-15s  %-22s  %-12s  🔴 STALE (cadence %ds, grace 25%%)\n' "$agent" "$last_iso" "$age_h" "$cadence_s"
        stale_count=$((stale_count + 1))
    else
        printf '%-15s  %-22s  %-12s  ✅ alive\n' "$agent" "$last_iso" "$age_h"
        alive_count=$((alive_count + 1))
    fi
done

if (( alive_count == 0 && stale_count == 0 )); then
    echo "(no heartbeats yet — agents have not had a chance to write one)" >&2
    exit 0
fi

echo
echo "Summary: $alive_count alive, $stale_count stale (now=$now_iso)"

if (( stale_count > 0 )); then
    exit 1
fi
exit 0
