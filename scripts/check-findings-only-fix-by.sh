#!/usr/bin/env bash
# Sweep BUILD_STATUS.md for MERGED-FINDINGS-ONLY rows and check
# their `fix-by YYYY-MM-DD` deadline against today's date
# (claude-conductor-fix-by-enforcement).
#
# Wire format: rows that ship `#[ignore]`'d regression tests with
# no actual fix MUST be flipped to status MERGED-FINDINGS-ONLY,
# and their note SHOULD carry a `fix-by YYYY-MM-DD` token naming
# when a fix-PR is expected on main. Rows without `fix-by` are
# back-compat: treated as "no deadline" / informational only.
#
# This sweep is advisory — the conductor's tick consumes the
# output and surfaces it in the summary; the script exits 1 ONLY
# when ≥1 overdue row is found, so a CI / cron caller can use the
# exit code to escalate. Within-budget + due-today + no-deadline
# all exit 0.
#
# Usage:
#   ./scripts/check-findings-only-fix-by.sh [--ledger PATH] [--today YYYY-MM-DD]
#
# Output (human-readable):
#   one line per MERGED-FINDINGS-ONLY row:
#     🟢 <brief-id>                       (informational, ok)
#     🟡 <brief-id>  due today  (YYYY-MM-DD)
#     🔴 <brief-id>  overdue    (YYYY-MM-DD, N days)
#   followed by a tally line.
#
# Exit codes:
#   0   no overdue rows (within-budget / due-today / no-deadline)
#   1   ≥1 overdue row found
#   2   prerequisite missing (ledger file unreadable) OR --today malformed

set -uo pipefail

LEDGER=""
TODAY=""
for arg in "$@"; do
  case "$arg" in
    --ledger=*) LEDGER="${arg#*=}" ;;
    --today=*)  TODAY="${arg#*=}" ;;
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

if [ -z "$TODAY" ]; then
  TODAY="$(date -u +%Y-%m-%d)"
fi

# Validate the today date — guard against `--today not-a-date`
# accidentally turning every row green.
if ! [[ "$TODAY" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  echo "FAIL: --today expects YYYY-MM-DD, got '$TODAY'" >&2
  exit 2
fi

# Normalize a date token to YYYY-MM-DD. Accepts the canonical
# form and two common operator-typed variants:
#   2026-05-25  (canonical)
#   2026/05/25  (slashes)
#   25 May 2026 (D Mon YYYY)
# Unparseable → empty string; caller treats that as "no deadline".
normalize_date() {
  local raw="$1"
  if [[ "$raw" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
    echo "$raw"
    return
  fi
  if [[ "$raw" =~ ^([0-9]{4})/([0-9]{2})/([0-9]{2})$ ]]; then
    echo "${BASH_REMATCH[1]}-${BASH_REMATCH[2]}-${BASH_REMATCH[3]}"
    return
  fi
  if [[ "$raw" =~ ^([0-9]{1,2})\ ([A-Za-z]{3})\ ([0-9]{4})$ ]]; then
    local d="${BASH_REMATCH[1]}" m_abbr="${BASH_REMATCH[2]}" y="${BASH_REMATCH[3]}"
    local m=""
    case "$(echo "$m_abbr" | tr '[:upper:]' '[:lower:]')" in
      jan) m=01 ;; feb) m=02 ;; mar) m=03 ;; apr) m=04 ;;
      may) m=05 ;; jun) m=06 ;; jul) m=07 ;; aug) m=08 ;;
      sep) m=09 ;; oct) m=10 ;; nov) m=11 ;; dec) m=12 ;;
    esac
    if [ -n "$m" ]; then
      printf '%s-%s-%02d\n' "$y" "$m" "$d"
      return
    fi
  fi
  echo ""
}

# Day-difference: (lhs_date - rhs_date) in days. Uses bash arith
# on epoch seconds parsed via `date -j` (BSD/macOS) with `date -d`
# (GNU/Linux) as fallback. Stays portable across the workstations
# we run on.
days_between() {
  local a="$1" b="$2"
  local a_secs b_secs
  if a_secs=$(date -j -f "%Y-%m-%d" "$a" +%s 2>/dev/null) \
     && b_secs=$(date -j -f "%Y-%m-%d" "$b" +%s 2>/dev/null); then
    : # macOS path
  else
    a_secs=$(date -d "$a" +%s 2>/dev/null) || { echo 0; return; }
    b_secs=$(date -d "$b" +%s 2>/dev/null) || { echo 0; return; }
  fi
  echo $(((a_secs - b_secs) / 86400))
}

# Pull `MERGED-FINDINGS-ONLY` rows from the ledger. We anchor on
# the third column (status) to avoid pattern-matching the literal
# string inside narrative note text on already-MERGED rows. The
# awk filter splits on `|`, trims the status field, and writes
# `<brief-id>\t<full-reason>` for each match.
rows=$(awk -F'|' '
  /^\| [a-zA-Z0-9_.-]+/ {
    status=$4; gsub(/^ +| +$/, "", status)
    if (status == "MERGED-FINDINGS-ONLY") {
      id=$2; gsub(/^ +| +$/, "", id)
      reason=$6; gsub(/^ +| +$/, "", reason)
      print id "\t" reason
    }
  }' "$LEDGER")

green=0
yellow=0
red=0
informational=0

if [ -z "$rows" ]; then
  echo "No MERGED-FINDINGS-ONLY rows in ledger — nothing to sweep."
  exit 0
fi

# Process each row in a here-string so the green/yellow/red
# tallies survive the loop (avoids subshell-on-pipe).
while IFS=$'\t' read -r brief_id reason; do
  [ -z "$brief_id" ] && continue
  # Extract the fix-by token. Tolerates surrounding markdown
  # punctuation; matches `fix-by 2026-05-25` and the slash/word
  # variants we normalize below.
  fix_by_raw=""
  if [[ "$reason" =~ fix-by[[:space:]]+([0-9]{4}-[0-9]{2}-[0-9]{2}|[0-9]{4}/[0-9]{2}/[0-9]{2}|[0-9]{1,2}[[:space:]][A-Za-z]{3}[[:space:]][0-9]{4}) ]]; then
    fix_by_raw="${BASH_REMATCH[1]}"
  fi
  if [ -z "$fix_by_raw" ]; then
    echo "🟢 $brief_id                       (no fix-by deadline)"
    informational=$((informational + 1))
    continue
  fi
  fix_by=$(normalize_date "$fix_by_raw")
  if [ -z "$fix_by" ]; then
    echo "🟢 $brief_id                       (fix-by token '$fix_by_raw' unparseable, treated as no deadline)"
    informational=$((informational + 1))
    continue
  fi
  diff=$(days_between "$TODAY" "$fix_by")
  if [ "$diff" -lt 0 ]; then
    echo "🟢 $brief_id  within budget ($fix_by, $((-diff)) days remaining)"
    green=$((green + 1))
  elif [ "$diff" -eq 0 ]; then
    echo "🟡 $brief_id  due today  ($fix_by)"
    yellow=$((yellow + 1))
  else
    echo "🔴 $brief_id  overdue    ($fix_by, $diff days past)"
    red=$((red + 1))
  fi
done <<< "$rows"

total=$((green + yellow + red + informational))
echo
echo "Tally: $green within-budget · $yellow due-today · $red overdue · $informational informational (no deadline) · $total total."

if [ "$red" -gt 0 ]; then
  exit 1
fi
exit 0
