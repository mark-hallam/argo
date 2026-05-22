#!/usr/bin/env bash
# scripts/check-unread-handoffs.sh
#
# Surfaces docs/skycot/agents/SHARED.md entries from non-conductor
# agents posted after the conductor's last_run_utc heartbeat.
#
# Purpose: prevent the failure mode where the evaluator (or auditor,
# marketing, etc.) lands a handoff entry between conductor ticks and
# the conductor doesn't action it. The conductor's tick walks recent
# evaluator FILES, but the SHARED.md cross-agent ledger is the
# canonical "you have new mail" channel — this script makes that
# explicit.
#
# Exit codes:
#   0  no unread handoffs from non-conductor agents
#   1  >=1 unread handoff from a non-conductor agent (BLOCK the tick
#      from declaring complete until each is actioned)
#   2  internal / parse error (proceed cautiously)
#
# Usage:
#   bash scripts/check-unread-handoffs.sh                # default
#   bash scripts/check-unread-handoffs.sh --since <ISO>  # override
#
# Wired into docs/build-plan/PROTOCOL_LOOP_PROMPT.md step 4.5.

set -euo pipefail

REPO="${ARGO_REPO_ROOT:-$(pwd)}"
SHARED="$REPO/docs/skycot/agents/SHARED.md"
HEARTBEAT="$REPO/docs/skycot/agents/heartbeats/conductor.json"

# Resolve the cutoff timestamp.
if [ "${1:-}" = "--since" ] && [ -n "${2:-}" ]; then
  CUTOFF_UTC="$2"
elif [ -f "$HEARTBEAT" ]; then
  CUTOFF_UTC=$(python3 -c "
import json, sys
try:
    d = json.load(open('$HEARTBEAT'))
    print(d.get('last_run_utc', ''))
except Exception as e:
    print('', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null || echo "")
else
  CUTOFF_UTC=""
fi

if [ -z "$CUTOFF_UTC" ]; then
  echo "ℹ️  no conductor heartbeat found — treating all SHARED.md entries as unread"
  CUTOFF_EPOCH=0
else
  CUTOFF_EPOCH=$(python3 -c "
from datetime import datetime
try:
    ts = '$CUTOFF_UTC'.replace('Z', '+00:00')
    print(int(datetime.fromisoformat(ts).timestamp()))
except Exception as e:
    print('CUTOFF_PARSE_ERROR:', e, file=__import__('sys').stderr)
    __import__('sys').exit(2)
")
fi

if [ ! -f "$SHARED" ]; then
  echo "ℹ️  no SHARED.md at $SHARED — nothing to check"
  exit 0
fi

# Walk SHARED.md. Each tracked entry begins with an ISO 8601 timestamp
# followed by [agent] tag. We tolerate the legacy "Z+1000" suffix that
# a few conductor entries use — strip the offset for the epoch calc.
UNREAD_COUNT=0
UNREAD_BUFFER=""

while IFS= read -r line; do
  # Match: 2026-05-23T04:10:00Z [evaluator] round 5 — ...
  # Also:  2026-05-22T06:55:00Z+1000 [conductor] ...   (legacy)
  if [[ "$line" =~ ^([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2})Z?(\+[0-9]{4})?\ \[([a-z-]+)\] ]]; then
    ts="${BASH_REMATCH[1]}Z"   # normalize to UTC
    agent="${BASH_REMATCH[3]}"

    # Conductor talking to itself is not a handoff.
    if [ "$agent" = "conductor" ]; then
      continue
    fi

    line_epoch=$(python3 -c "
from datetime import datetime
try:
    print(int(datetime.fromisoformat('${ts}'.replace('Z', '+00:00')).timestamp()))
except Exception:
    print(0)
" 2>/dev/null || echo 0)

    if [ "$line_epoch" -gt "$CUTOFF_EPOCH" ]; then
      UNREAD_COUNT=$((UNREAD_COUNT + 1))
      UNREAD_BUFFER="${UNREAD_BUFFER}  ${line}"$'\n'
    fi
  fi
done < "$SHARED"

if [ "$UNREAD_COUNT" -gt 0 ]; then
  printf '\033[1;31m🔴 UNREAD HANDOFFS (%d) in SHARED.md, newer than conductor cutoff %s:\033[0m\n\n' \
    "$UNREAD_COUNT" "$CUTOFF_UTC"
  printf '%s\n' "$UNREAD_BUFFER"
  cat <<'EOF'
ACTION REQUIRED before this tick can declare complete:
  1. For each entry above, decide: author a brief / flip ledger row / surface to operator.
  2. Once all are actioned, Step 4.6 will refresh the heartbeat — that "marks them read."
  3. If an entry is informational only (status, no action needed), still note it in the tick summary.

EOF
  exit 1
fi

printf '\033[1;32m✅ No unread handoffs from other agents (cutoff %s).\033[0m\n' "$CUTOFF_UTC"
exit 0
