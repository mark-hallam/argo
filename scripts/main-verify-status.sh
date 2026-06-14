#!/usr/bin/env bash
# main-verify-status.sh — canonical reader of the post-merge sweep state.
# Prints a one-line human summary + the full status.json. Read-only.
set -uo pipefail
DIR="$HOME/.argo/main-verify"
STATUS="$DIR/status.json"
FREEZE="$DIR/merge-freeze"
if [ ! -f "$STATUS" ]; then
  echo "main-verify: NO STATE YET (sweep has not run, or not installed). freeze=$([ -f "$FREEZE" ] && echo ON || echo off)"
  exit 0
fi
state="$(grep -oE '"state"[[:space:]]*:[[:space:]]*"[^"]*"' "$STATUS" | head -1 | grep -oE '"[^"]*"$' | tr -d '"')"
sha="$(grep -oE '"last_checked_sha"[[:space:]]*:[[:space:]]*"[^"]*"' "$STATUS" | head -1 | grep -oE '"[^"]*"$' | tr -d '"')"
at="$(grep -oE '"checked_at_utc"[[:space:]]*:[[:space:]]*"[^"]*"' "$STATUS" | head -1 | grep -oE '"[^"]*"$' | tr -d '"')"
echo "main-verify: ${state:-?} @ origin/main ${sha:0:9} (checked ${at:-?} UTC)  freeze=$([ -f "$FREEZE" ] && echo ON || echo off)"
echo "--- $STATUS ---"
cat "$STATUS"
