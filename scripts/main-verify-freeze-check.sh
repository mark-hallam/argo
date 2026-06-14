#!/usr/bin/env bash
# main-verify-freeze-check.sh — the conductor's pre-merge gate calls this.
# Exit 0 = clear to merge. Exit 1 = MERGE FREEZE active (origin/main is RED per
# the post-merge sweep); do NOT merge until the sweep flips it back to green.
set -uo pipefail
FREEZE="$HOME/.argo/main-verify/merge-freeze"
STATUS="$HOME/.argo/main-verify/status.json"
if [ -f "$FREEZE" ]; then
  echo "[main-verify] MERGE FREEZE ACTIVE — origin/main is RED. Do NOT admin-merge until green." >&2
  [ -f "$STATUS" ] && { echo "--- status.json ---" >&2; cat "$STATUS" >&2; }
  exit 1
fi
exit 0
