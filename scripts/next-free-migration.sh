#!/usr/bin/env bash
# next-free-migration.sh — find the next available migration slot.
#
# Migration slots are numbered 1..99. The argo-store crate has two parallel
# ladders that must stay in sync:
#   crates/argo-store/migrations/<NN>__<name>.sql   — raw SQL ladder
#   crates/argo-store/src/migrations/v<NN>.rs       — versioned-module ladder
#
# Brief authors and workers must claim a slot that's free in BOTH ladders.
# Tonight's v41 near-miss (claude-skill-share-cross-tenant-poc + claude-
# skill-canary-deployment + claude-worker-team-roster-cli) showed the
# manual coordination doesn't scale.
#
# This script reports:
#   - the next free slot (max occupied + 1, considering BOTH ladders)
#   - the next 5 free slots (in case the user wants headroom)
#   - any in-flight slots from CLAIMED/IN_REVIEW briefs (advisory; these
#     are not yet on main but are imminent)
#
# Usage:
#   scripts/next-free-migration.sh                # report next free slot
#   scripts/next-free-migration.sh --json         # machine-readable
#   scripts/next-free-migration.sh --reserve <brief-id>  # (advisory) print
#                                                          a brief-row stub
#                                                          claiming the slot

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RAW_DIR="$REPO_ROOT/crates/argo-store/migrations"
VER_DIR="$REPO_ROOT/crates/argo-store/src/migrations"
LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"

MODE="text"
RESERVE_BRIEF=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json) MODE="json"; shift ;;
        --reserve)
            MODE="reserve"
            RESERVE_BRIEF="${2:-}"
            shift 2
            ;;
        --help|-h)
            sed -n '2,29p' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

# Collect occupied slots from both ladders. Use space-delimited string as a
# set substitute so this works under bash 3.2 (macOS system bash).
raw_slots_list=$(ls "$RAW_DIR"/[0-9][0-9]__*.sql 2>/dev/null \
                   | sed -E 's@.*/([0-9]+)__.*@\1@' | sort -un | tr '\n' ' ')
ver_slots_list=$(ls "$VER_DIR"/v[0-9][0-9].rs 2>/dev/null \
                   | sed -E 's@.*/v([0-9]+)\.rs@\1@' | sort -un | tr '\n' ' ')

raw_count=$(echo "$raw_slots_list" | tr ' ' '\n' | grep -c '^[0-9]')
ver_count=$(echo "$ver_slots_list" | tr ' ' '\n' | grep -c '^[0-9]')

# Helper: is slot $1 in the space-delimited set $2?
in_set() {
    case " $2 " in
        *" $1 "*) return 0 ;;
        *) return 1 ;;
    esac
}

# In-flight slots from CLAIMED/IN_REVIEW briefs that mention migration N.
in_flight_list=""
in_flight_count=0
if [[ -f "$LEDGER" ]]; then
    while IFS= read -r row; do
        case "$row" in
            *"| CLAIMED |"* | *"| IN_REVIEW |"*) ;;
            *) continue ;;
        esac
        # extract numeric slot mentions; restrict to 10-99 range
        for slot in $(echo "$row" | grep -oE 'migration[s]?[/[:space:]]*[vV]?[0-9]{1,3}' \
                                  | grep -oE '[0-9]{1,3}'); do
            if [[ "$slot" -ge 10 && "$slot" -le 99 ]] && ! in_set "$slot" "$in_flight_list"; then
                in_flight_list="$in_flight_list $slot"
                in_flight_count=$((in_flight_count + 1))
            fi
        done
    done < "$LEDGER"
fi

# Find next 6 free slots (skip both occupied and in-flight)
free_slots=""
free_count=0
slot=10
while [[ "$free_count" -lt 6 && "$slot" -le 99 ]]; do
    if ! in_set "$slot" "$raw_slots_list" \
       && ! in_set "$slot" "$ver_slots_list" \
       && ! in_set "$slot" "$in_flight_list"; then
        free_slots="$free_slots $slot"
        free_count=$((free_count + 1))
    fi
    slot=$((slot + 1))
done
free_slots=$(echo "$free_slots" | xargs)  # trim
next_free=$(echo "$free_slots" | awk '{print $1}')

# Report
case "$MODE" in
    json)
        # crude but valid JSON
        slots_arr=$(echo "$free_slots" | awk '{
            for (i = 1; i <= NF && i <= 5; i++) {
                if (i > 1) printf ",";
                printf "%d", $i
            }
            for (j = i; j <= 5; j++) {
                if (j > 1) printf ",";
                printf "0"
            }
        }')
        # Distinct occupied = union; using count-of-distinct from raw|ver lists
        occ_count=$(echo "$raw_slots_list $ver_slots_list" | tr ' ' '\n' \
                      | grep -E '^[0-9]+$' | sort -u | wc -l | xargs)
        printf '{"next_free": %d, "next_5_free": [%s], "occupied_count": %d, "in_flight_count": %d}\n' \
            "${next_free:-0}" "$slots_arr" "$occ_count" "$in_flight_count"
        ;;
    reserve)
        if [[ -z "$RESERVE_BRIEF" ]]; then
            echo "FAIL: --reserve requires a brief-id" >&2
            exit 2
        fi
        if [[ -z "$next_free" ]]; then
            echo "FAIL: no free slots in 10-99 range" >&2
            exit 1
        fi
        next="$next_free"
        cat <<EOF
Reserving slot $next for $RESERVE_BRIEF.

Add to your brief's "## Files you may touch" section:
  - \`crates/argo-store/migrations/${next}__$(echo "$RESERVE_BRIEF" | tr '-' '_').sql\` (NEW)
  - \`crates/argo-store/src/migrations/v${next}.rs\` (NEW — versioned module)

Add to "## Approach (concrete)" section:
  Migration v${next} wraps raw \`${next}__*.sql\`. Verified next free
  slot via scripts/next-free-migration.sh on $(date -u +%Y-%m-%d).

NOTE: This is ADVISORY. The slot is not atomically reserved. Race-loss
to another brief author is possible if you don't author quickly. To
reduce race-loss risk, ship the brief authoring commit within 30 min
of running this command, then re-run this script before any worker
claims the brief to confirm the slot is still free.
EOF
        ;;
    text|*)
        occ_count=$(echo "$raw_slots_list $ver_slots_list" | tr ' ' '\n' \
                      | grep -E '^[0-9]+$' | sort -u | wc -l | xargs)
        cat <<EOF
=== Migration slot ledger ===

Occupied (on origin/main):     $occ_count distinct slots (raw=$raw_count, versioned=$ver_count)
In-flight (CLAIMED/IN_REVIEW): $in_flight_count slot(s)

Next free slot: ${next_free:-NONE}
Next 5 free:    $free_slots

Usage in a brief:
  scripts/next-free-migration.sh --reserve <your-brief-id>

In-flight slots (advisory — not on main yet):
EOF
        if [[ "$in_flight_count" -eq 0 ]]; then
            echo "  (none)"
        else
            echo "$in_flight_list" | tr ' ' '\n' | grep -E '^[0-9]+$' | sort -un | sed 's/^/  slot /'
        fi
        ;;
esac
