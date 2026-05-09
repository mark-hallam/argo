#!/usr/bin/env bash
# append-history.sh — append a row-history record to BUILD_STATUS_HISTORY.ndjson.
#
# Reduces the load on BUILD_STATUS.md row notes. The current pattern of
# accumulating hundreds of LOC of historical notes per row makes the
# ledger unreadable and creates merge churn (every status flip races on
# nearby rows whose notes are growing).
#
# New convention (forward-only):
#   - BUILD_STATUS.md row notes ≤ 2 sentences, current state only
#   - Historical notes appended via this script to BUILD_STATUS_HISTORY.ndjson
#   - Old rows with long notes are NOT migrated — they stay as-is until
#     naturally rewritten (next status transition that touches the row)
#
# Each line of BUILD_STATUS_HISTORY.ndjson is one JSON record:
#   { "ts": "<ISO-8601 UTC>", "brief": "<brief-id>", "actor": "<host-or-conductor>",
#     "transition": "<from>→<to>", "pr": "<#NNNN-or-null>", "note": "<full note>" }
#
# Usage:
#   scripts/append-history.sh \
#     --brief claude-foo --actor mac-host-claude \
#     --from CLAIMED --to IN_REVIEW --pr 1234 \
#     --note "Worker rebased onto current main, ..."
#
# Quoting: --note may contain shell metacharacters; pass it as a single arg.
# The script JSON-escapes the note before writing.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HISTORY="$REPO_ROOT/docs/build-plan/BUILD_STATUS_HISTORY.ndjson"

BRIEF=""
ACTOR=""
FROM=""
TO=""
PR=""
NOTE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --brief) BRIEF="${2:-}"; shift 2 ;;
        --actor) ACTOR="${2:-}"; shift 2 ;;
        --from)  FROM="${2:-}";  shift 2 ;;
        --to)    TO="${2:-}";    shift 2 ;;
        --pr)    PR="${2:-}";    shift 2 ;;
        --note)  NOTE="${2:-}";  shift 2 ;;
        --help|-h) sed -n '2,28p' "$0" | sed 's/^# \?//'; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [[ -z "$BRIEF" || -z "$ACTOR" || -z "$FROM" || -z "$TO" ]]; then
    echo "FAIL: --brief, --actor, --from, --to are required" >&2
    exit 2
fi

# Ensure history file exists with a header comment line (NDJSON tolerates blank
# lines but we use a leading "// ..." commented-out line as a one-time header).
if [[ ! -f "$HISTORY" ]]; then
    cat > "$HISTORY" <<'EOF'
// BUILD_STATUS_HISTORY.ndjson — append-only log of ledger row transitions.
// Each subsequent line is a single JSON object; this header line is ignored.
// Format: {"ts","brief","actor","transition","pr","note"}
EOF
fi

# JSON-escape the note via Python (handles unicode, quotes, newlines, etc.)
escaped_note=$(python3 -c '
import json, sys
print(json.dumps(sys.argv[1]))
' "$NOTE")

ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
pr_field=$([[ -z "$PR" ]] && echo "null" || echo "\"#$PR\"")
escaped_brief=$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$BRIEF")
escaped_actor=$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$ACTOR")
escaped_transition=$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$FROM→$TO")

printf '{"ts":"%s","brief":%s,"actor":%s,"transition":%s,"pr":%s,"note":%s}\n' \
    "$ts" "$escaped_brief" "$escaped_actor" "$escaped_transition" \
    "$pr_field" "$escaped_note" \
    >> "$HISTORY"

echo "appended: $BRIEF $FROM→$TO $(${PR:+pr=#$PR })" >&2
