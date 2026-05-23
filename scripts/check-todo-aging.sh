#!/usr/bin/env bash
# Sweep BUILD_STATUS.md for unfinished P0 rows and report how long
# each has been waiting. "Unfinished" = `status` is one of TODO /
# CLAIMED / CHANGES-REQUESTED — anything not yet on main.
#
# Why: scheduled audit 2026-05-23T09-40 §4 P1 #4 — the cu-macos
# P0 went 17h+ unfixed during the mac-host doom-loop because the
# sibling `scripts/check-findings-only-fix-by.sh` (from
# claude-conductor-fix-by-enforcement / #2065) only walks rows
# that are MERGED-FINDINGS-ONLY, not pre-merge rows. A P0 that's
# been sitting in TODO / CLAIMED / CHANGES-REQUESTED for >24h is
# operationally indistinguishable from a P0 that's been forgotten
# about. This script closes that gap (initial TODO-only landing
# via ops-add-todo-aging-gate-script / #2134, broadened to
# CLAIMED + CHANGES-REQUESTED via
# ops-fix-by-tracker-add-p0-cu-macos-blocker — same shape, wider
# net so an actively-CLAIMED-but-not-shipping P0 also escalates).
#
# The output mirrors `check-findings-only-fix-by.sh` so the
# conductor's tick surfaces both gates with the same visual
# language.
#
# Usage:
#   ./scripts/check-todo-aging.sh [--ledger PATH] [--now YYYY-MM-DDTHH:MMZ]
#
# Output (human-readable):
#   one line per P0 row in TODO / CLAIMED / CHANGES-REQUESTED:
#     🟢 <brief-id>  within budget (<H>h since <anchor>)
#     🟠 <brief-id>  aging         (<H>h since <anchor>)
#     🔴 <brief-id>  OVERDUE       (<H>h since <anchor>)
#   followed by a tally line.
#
# Exit codes (informational only — never escalates):
#   0   always
#   2   prerequisite missing (ledger / --now unparseable)

set -uo pipefail

LEDGER=""
NOW=""
for arg in "$@"; do
  case "$arg" in
    --ledger=*) LEDGER="${arg#*=}" ;;
    --now=*)    NOW="${arg#*=}" ;;
    --help|-h)
      sed -n '2,/^$/p' "$0" | sed 's/^#\s\?//'
      exit 0
      ;;
    *) echo "FAIL: unknown arg '$arg' (try --help)" >&2; exit 2 ;;
  esac
done

if [ -z "$LEDGER" ]; then
  REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
  LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"
fi

if [ ! -f "$LEDGER" ]; then
  echo "FAIL: ledger not found at $LEDGER" >&2
  exit 2
fi

if [ -z "$NOW" ]; then
  NOW="$(date -u +%Y-%m-%dT%H:%MZ)"
fi

# Validate --now — guard against typos turning every row green.
if ! [[ "$NOW" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}Z$ ]]; then
  echo "FAIL: --now expects YYYY-MM-DDTHH:MMZ, got '$NOW'" >&2
  exit 2
fi

# Convert YYYY-MM-DDTHH:MMZ to epoch seconds (UTC). macOS BSD
# `date -j -u -f` and GNU `date -u -d` paths supported. `-u`
# placement and `+%s` ordering matters on BSD; out-of-order args
# silently return the parsed-back human form instead of an
# integer (and break downstream arithmetic).
to_epoch() {
  local stamp="$1"
  local secs
  # macOS / BSD: `date -j -u -f <fmt> <input> +<output>`. The
  # `+` token must be the LAST positional arg.
  if secs=$(date -j -u -f "%Y-%m-%dT%H:%MZ" "$stamp" +%s 2>/dev/null); then
    # Only accept all-digits output; reject the BSD-on-bad-args
    # human-readable fallback.
    if [[ "$secs" =~ ^[0-9]+$ ]]; then
      echo "$secs"
      return
    fi
  fi
  # GNU coreutils: `date -u -d <input> +<output>`.
  if secs=$(date -u -d "$stamp" +%s 2>/dev/null); then
    if [[ "$secs" =~ ^[0-9]+$ ]]; then
      echo "$secs"
      return
    fi
  fi
  echo ""
}

NOW_SECS=$(to_epoch "$NOW")
if [ -z "$NOW_SECS" ]; then
  echo "FAIL: cannot parse --now timestamp '$NOW'" >&2
  exit 2
fi

# Pull TODO rows tagged with a P0 token in the note. Anchored on
# the status column (column 4) to avoid pattern-matching against
# narrative text in already-MERGED rows. Recognised P0 markers
# (structural — `P0` alone in prose doesn't count, the brief's
# convention is one of these specific shapes):
#   - 🔴 P0  / 🚨 P0  (severity-emoji prefix)
#   - **P0   (bold-P0 inside a markdown emphasis block)
#   - P0:    (colon-suffixed prefix)
#   - OPERATOR-LOCKED (always escalated)
rows=$(awk -F'|' '
  /^\| [a-zA-Z0-9_.-]+/ {
    status=$4; gsub(/^ +| +$/, "", status)
    # Pre-merge statuses only — anything that has landed on main
    # (MERGED, MERGED-FINDINGS-ONLY, MERGED-DUP-OF-*, DROPPED,
    # DEFERRED-*) is out of scope for this sweep. The fix-by sweep
    # at step 4.8 handles MERGED-FINDINGS-ONLY separately.
    if (status != "TODO" && \
        status != "CLAIMED" && \
        status != "CHANGES-REQUESTED") next
    reason=$6; gsub(/^ +| +$/, "", reason)
    id=$2; gsub(/^ +| +$/, "", id)
    if (reason ~ /🔴 P0/ || \
        reason ~ /🚨 P0/ || \
        reason ~ /\*\*P0/ || \
        reason ~ /P0:/ || \
        reason ~ /OPERATOR-LOCKED/) {
      print id "\t" reason
    }
  }' "$LEDGER")

green=0
orange=0
red=0
total=0

if [ -z "$rows" ]; then
  echo "No P0-tagged TODO / CLAIMED / CHANGES-REQUESTED rows in ledger — nothing to age."
  exit 0
fi

# For each row, extract the most-recent `YYYY-MM-DDTHH:MMZ` token.
# Brief says: NEW <date>, claimed at <date>, RESTORED <date> are
# the anchor candidates. We extract every well-formed stamp from
# the note and pick the latest one as the "anchor" (most recent
# touch).
while IFS=$'\t' read -r brief_id reason; do
  [ -z "$brief_id" ] && continue
  total=$((total + 1))
  # Anchor candidates, in order of precision:
  #   1. ISO `2026-MM-DDTHH:MMZ` (most precise — claimed-at /
  #      MERGED-at / IN_REVIEW-at stamps the swarm writes).
  #   2. Date-only `2026-MM-DD` (NEW <date> on briefs that don't
  #      carry a time). Treated as midnight UTC of that date.
  # If both shapes appear, the ISO stamps win because they're
  # always at least as recent as the date-only NEW marker.
  iso_stamps=$(echo "$reason" | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}Z' | sort -u)
  date_only_stamps=$(echo "$reason" | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' | sort -u)
  latest=""
  for s in $iso_stamps; do
    if [ -z "$latest" ] || [[ "$s" > "$latest" ]]; then
      latest="$s"
    fi
  done
  if [ -z "$latest" ]; then
    for s in $date_only_stamps; do
      # Promote date-only to midnight-UTC ISO so the downstream
      # to_epoch path is uniform.
      iso="${s}T00:00Z"
      if [ -z "$latest" ] || [[ "$iso" > "$latest" ]]; then
        latest="$iso"
      fi
    done
  fi
  if [ -z "$latest" ]; then
    # Final fallback: git log first-introduction date of the
    # ledger (not the row — git's `--follow` won't track row
    # additions inside a single file). Imprecise but a backstop
    # so the script never emits a blank anchor.
    if first_seen=$(git -C "$(dirname "$LEDGER")/../.." log --diff-filter=A --follow --format=%cI -- "$LEDGER" 2>/dev/null | tail -1); then
      latest=$(echo "$first_seen" | sed -E 's/([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}).*/\1Z/')
    fi
  fi
  if [ -z "$latest" ]; then
    echo "🟢 $brief_id  no anchor timestamp found (informational)"
    green=$((green + 1))
    continue
  fi
  anchor_secs=$(to_epoch "$latest")
  if [ -z "$anchor_secs" ]; then
    echo "🟢 $brief_id  anchor '$latest' unparseable (informational)"
    green=$((green + 1))
    continue
  fi
  age_hours=$(( (NOW_SECS - anchor_secs) / 3600 ))
  if [ "$age_hours" -lt 0 ]; then
    # Anchor in the future (clock skew or seeded fixture) — treat
    # as informational, don't crash on negative duration.
    echo "🟢 $brief_id  anchor '$latest' is in the future (clock skew?)"
    green=$((green + 1))
    continue
  fi
  if [ "$age_hours" -lt 24 ]; then
    echo "🟢 $brief_id  within budget (${age_hours}h since $latest)"
    green=$((green + 1))
  elif [ "$age_hours" -lt 48 ]; then
    echo "🟠 $brief_id  aging         (${age_hours}h since $latest)"
    orange=$((orange + 1))
  else
    echo "🔴 $brief_id  OVERDUE       (${age_hours}h since $latest)"
    red=$((red + 1))
  fi
done <<< "$rows"

echo
echo "Tally: $green within-budget · $orange aging · $red overdue · $total total."
# Exit 0 always — informational, never escalates. Conductor's tick
# consumes the output and decides whether to act.
exit 0
