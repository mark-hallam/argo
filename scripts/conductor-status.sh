#!/usr/bin/env bash
# conductor-status.sh — at-a-glance swarm status for the conductor seat.
#
# Reconstructs ledger + activity state from BUILD_STATUS.md + git log so
# the conductor doesn't burn context re-reading the ledger every tick.
#
# Usage:
#   scripts/conductor-status.sh                # current snapshot
#   scripts/conductor-status.sh --since 24h    # also show activity timeline
#   scripts/conductor-status.sh --since 6h --verbose   # include held PRs detail
#   scripts/conductor-status.sh --json         # machine-readable summary
#
# Sections:
#   1. Queue counts (TODO / CLAIMED / IN_REVIEW / BLOCKED / MERGED)
#   2. Stale-CLAIMED candidates (claim age > threshold, no PR)
#   3. Stale-IN_REVIEW candidates (last push > 2h, fix-request comment present)
#   4. (--since): activity timeline — merges, sweeps, holds, claim-fails
#   5. (--verbose): held PR details with fix-request excerpts

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"
SINCE=""
MODE="text"
VERBOSE=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --since) SINCE="${2:-}"; shift 2 ;;
        --since=*) SINCE="${1#--since=}"; shift ;;
        --json) MODE="json"; shift ;;
        --verbose|-v) VERBOSE=1; shift ;;
        --help|-h) sed -n '2,15p' "$0" | sed 's/^# \?//'; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

cd "$REPO_ROOT"

# Section 1: queue counts (claude-side only; codex is offline)
counts=$(awk -F'|' '
  /\| (claude|codex)-/ {
    gsub(/^[ \t]+|[ \t]+$/, "", $3)
    gsub(/^[ \t]+|[ \t]+$/, "", $4)
    if ($3 == "claude") c[$4]++
  }
  END { for (k in c) printf "%s=%d ", k, c[k] }
' "$LEDGER")

todo=$(echo "$counts" | tr ' ' '\n' | grep '^TODO=' | cut -d= -f2)
claimed=$(echo "$counts" | tr ' ' '\n' | grep '^CLAIMED=' | cut -d= -f2)
in_review=$(echo "$counts" | tr ' ' '\n' | grep '^IN_REVIEW=' | cut -d= -f2)
blocked=$(echo "$counts" | tr ' ' '\n' | grep '^BLOCKED=' | cut -d= -f2)
merged=$(echo "$counts" | tr ' ' '\n' | grep '^MERGED=' | cut -d= -f2)

# Section 2: stale-CLAIMED candidates
NOW_EPOCH=$(date +%s)
STALE_CLAIM_THRESHOLD_MIN=90  # 1h30m, matches af500037 spirit

stale_claims=""
while IFS= read -r brief; do
    [[ -z "$brief" ]] && continue
    ts=$(git log --format="%at" --grep="claim $brief\$" -1 2>/dev/null | head -1)
    [[ -z "$ts" ]] && continue
    age_min=$(( (NOW_EPOCH - ts) / 60 ))
    if [[ "$age_min" -gt "$STALE_CLAIM_THRESHOLD_MIN" ]]; then
        stale_claims="$stale_claims  ${age_min}min  $brief\n"
    fi
done < <(grep -E '^\| claude-.* \| claude \| CLAIMED \|' "$LEDGER" \
           | awk -F'|' '{gsub(/^ +| +$/,"",$2); print $2}')

# Section 3: stale-IN_REVIEW (>2h since last push, has fix-request comment)
stale_reviews=""
if command -v gh >/dev/null 2>&1; then
    while IFS= read -r row; do
        # Extract PR number
        pr=$(echo "$row" | awk -F'|' '{gsub(/^ +| +$/,"",$5); print $5}' | grep -oE '#[0-9]+' | head -1 | tr -d '#')
        [[ -z "$pr" ]] && continue
        brief=$(echo "$row" | awk -F'|' '{gsub(/^ +| +$/,"",$2); print $2}')
        update=$(gh pr view "$pr" --json updatedAt --jq '.updatedAt' 2>/dev/null)
        [[ -z "$update" ]] && continue
        update_ts=$(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$update" +%s 2>/dev/null \
                    || date -d "$update" +%s 2>/dev/null)
        [[ -z "$update_ts" ]] && continue
        age_min=$(( (NOW_EPOCH - update_ts) / 60 ))
        if [[ "$age_min" -gt 120 ]]; then
            stale_reviews="$stale_reviews  ${age_min}min  #$pr  $brief\n"
        fi
    done < <(grep -E '^\| claude-.* \| claude \| IN_REVIEW \|' "$LEDGER")
fi

# Output
case "$MODE" in
    json)
        printf '{"todo":%d,"claimed":%d,"in_review":%d,"blocked":%d,"merged":%d}\n' \
            "${todo:-0}" "${claimed:-0}" "${in_review:-0}" "${blocked:-0}" "${merged:-0}"
        ;;
    text|*)
        echo "=== Conductor status — $(date -u +%Y-%m-%dT%H:%MZ) ==="
        echo ""
        echo "Queue (claude-side):"
        printf "  TODO       %3d\n" "${todo:-0}"
        printf "  CLAIMED    %3d\n" "${claimed:-0}"
        printf "  IN_REVIEW  %3d\n" "${in_review:-0}"
        printf "  BLOCKED    %3d\n" "${blocked:-0}"
        printf "  MERGED     %3d\n" "${merged:-0}"
        if [[ "${todo:-0}" -le 8 ]]; then
            echo ""
            echo "  ⚠️  TODO ≤ 8 — replenishment threshold hit (per BUILD_STATUS hard rule)."
        fi
        echo ""
        if [[ -n "$stale_claims" ]]; then
            echo "Stale-CLAIMED (>${STALE_CLAIM_THRESHOLD_MIN}min, no PR — sweep candidates):"
            printf "$stale_claims"
        else
            echo "Stale-CLAIMED: none"
        fi
        echo ""
        if [[ -n "$stale_reviews" ]]; then
            echo "Stale-IN_REVIEW (>2h since last push — sweep candidates):"
            printf "$stale_reviews"
        else
            echo "Stale-IN_REVIEW: none"
        fi
        echo ""

        if [[ -n "$SINCE" ]]; then
            echo "Activity since $SINCE:"
            git log --since="$SINCE" --oneline origin/main 2>/dev/null \
              | grep -E '^[a-f0-9]+ (status:|.*\(#[0-9]+\)$)' | head -30 \
              | awk '
                  /merged/ || /\(#[0-9]+\) \(#/ { print "  ✓ MERGED "$0; next }
                  /demote|sweep|RECOVERED|RELEASED/ { print "  ⚠ SWEEP  "$0; next }
                  /claim/ { print "  • CLAIM  "$0; next }
                  /in review/ { print "  → IN_RV  "$0; next }
                  { print "  · OTHER  "$0 }
                '
            echo ""
        fi

        if [[ "$VERBOSE" -eq 1 && -n "$stale_reviews" ]]; then
            echo "Held PR details (verbose):"
            echo "$stale_reviews" | grep -oE '#[0-9]+' | sort -u | while read -r pr; do
                pr_num="${pr#\#}"
                title=$(gh pr view "$pr_num" --json title --jq '.title' 2>/dev/null | head -c 80)
                last_comment=$(gh pr view "$pr_num" --json comments \
                                 --jq '.comments[-1].body[:200]' 2>/dev/null \
                                 | head -c 200)
                echo "  $pr  $title"
                echo "    last comment: $last_comment"
            done
        fi
        ;;
esac
