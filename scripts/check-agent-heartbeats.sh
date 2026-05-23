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

# Agents whose heartbeat file may be split across multiple session
# files (e.g. evaluator-interactive.json + evaluator-scheduled.json).
# Source: ops-evaluator-heartbeat-concurrent-write-guard (round-31
# SHARED.md handoff). Two concurrent evaluator sessions race on a
# single evaluator.json; last-writer-wins overwrites the newer
# timestamp with an older one. The per-session-file split lets
# each session own its own file; the freshness check below
# collapses them back to one canonical agent for the staleness
# verdict, taking whichever timestamp is freshest.
#
# To add an agent to the allowlist: append its base name here, then
# document the per-session naming convention in the agent's role
# doc + heartbeats/README.md.
SPLIT_AGENTS=(evaluator)

canonical_agent_for() {
    local name="$1"
    local split
    for split in "${SPLIT_AGENTS[@]}"; do
        if [[ "$name" == "$split"-* ]]; then
            echo "$split"
            return
        fi
    done
    echo "$name"
}

stale_count=0
alive_count=0

printf '%-15s  %-22s  %-12s  %s\n' 'AGENT' 'LAST RUN (UTC)' 'AGE' 'STATUS'
printf '%-15s  %-22s  %-12s  %s\n' '─────' '─────────────────────' '────────────' '──────'

# Future-dated rejection threshold. Round-36 evaluator (2026-05-23)
# observed a live race where a concurrent session stamped Melbourne
# local time as `Z` (UTC), producing a `last_run_utc` 10h ahead of
# real UTC. With the per-session-file dedupe from #2139 in place,
# the future-dated row stays lexicographically-largest forever —
# wrong-time mode becomes silently permanent. Reject any timestamp
# >5min ahead of `now_iso`: tight enough to catch tz mistakes
# (typically multiple-hour offsets) and loose enough for clock
# drift between writer + reader on the same host.
FUTURE_DATED_GRACE_S=300

# First pass: collect (canonical_agent, last_iso, hb_path) for every
# heartbeat file, then keep only the freshest entry per canonical
# agent. This is the load-bearing concurrent-write guard: when two
# sessions write `evaluator-interactive.json` and
# `evaluator-scheduled.json` concurrently, the freshest still wins
# at staleness-check time. Rows whose timestamp is more than
# FUTURE_DATED_GRACE_S ahead of now are shunted to `reject_file`
# instead of the main collect stream so a wrong-time agent can't
# monopolise the freshness verdict.
shopt -s nullglob
collect_file="$(mktemp -t agent-heartbeats.XXXXXX)"
reject_file="$(mktemp -t agent-heartbeats-rej.XXXXXX)"
trap 'rm -f "$collect_file" "$collect_file.dedup" "$reject_file"' EXIT
for hb in "$HEARTBEAT_DIR"/*.json; do
    name="$(basename "$hb" .json)"
    [[ "$name" == "README" ]] && continue
    canonical="$(canonical_agent_for "$name")"
    candidate_iso="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('last_run_utc',''))" "$hb" 2>/dev/null || echo "")"
    # Future-dated guard. Convert the candidate to epoch and
    # compare to now_epoch. A timestamp more than
    # FUTURE_DATED_GRACE_S ahead of now goes to reject_file —
    # surfaced in the report as `⚠ FUTURE-DATED` so the operator
    # sees what was dropped, never silently swallowed.
    # Unparseable timestamps fall through to the normal path and
    # hit the existing MALFORMED HEARTBEAT render; no double-flag.
    if [[ -n "$candidate_iso" ]]; then
        if cand_epoch="$(date -u -j -f '%Y-%m-%dT%H:%M:%SZ' "$candidate_iso" +%s 2>/dev/null)" \
            || cand_epoch="$(date -u -d "$candidate_iso" +%s 2>/dev/null)"; then
            if (( cand_epoch - now_epoch > FUTURE_DATED_GRACE_S )); then
                printf '%s\t%s\t%s\n' "$canonical" "$candidate_iso" "$hb" >> "$reject_file"
                continue
            fi
        fi
    fi
    # Emit (canonical, last_iso, path) so the freshest survives the
    # sort below. Missing/empty last_iso sorts last so it only
    # surfaces if no sibling file has a valid timestamp.
    printf '%s\t%s\t%s\n' "$canonical" "${candidate_iso:-0000-00-00T00:00:00Z}" "$hb" >> "$collect_file"
done

# Sort by canonical asc, then by last_iso desc (lexicographic on
# the ISO-8601 timestamp is correct because the format is fixed-
# width). awk's `!seen[$1]++` keeps the first line per canonical
# — which after this sort is the freshest.
sort -k1,1 -k2,2r "$collect_file" | awk -F'\t' '!seen[$1]++' > "$collect_file.dedup"

while IFS=$'\t' read -r agent last_iso_candidate hb; do
    [[ -z "$agent" ]] && continue

    # `last_iso` comes from the pre-pass (already chosen as the
    # freshest among per-session files). Re-parse `cadence_s`
    # from the winning file — split-agent sessions all carry the
    # same cadence by convention.
    last_iso="$last_iso_candidate"
    cadence_s="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('expected_cadence_seconds',0))" "$hb")"

    if [[ -z "$last_iso" || "$last_iso" == "0000-00-00T00:00:00Z" || -z "$cadence_s" || "$cadence_s" == "0" ]]; then
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
done < "$collect_file.dedup"

# Surface every future-dated row that was shunted to reject_file.
# Each entry counts as STALE so the script exit code reflects the
# anomaly. If a canonical agent has ALL of its sibling heartbeats
# in reject_file (no entry survived to collect_file.dedup), the
# agent didn't render above — it appears here as 🔴 STALE on the
# future-dated line, the right answer.
while IFS=$'\t' read -r agent last_iso hb; do
    [[ -z "$agent" ]] && continue
    printf '%-15s  %-22s  %-12s  ⚠️  FUTURE-DATED (rejected, >5min ahead of now)\n' \
        "$agent" "$last_iso" "—"
    stale_count=$((stale_count + 1))
done < "$reject_file"

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
