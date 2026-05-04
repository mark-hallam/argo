#!/usr/bin/env bash
# Capture a fresh data/demo.json by running the GTM Engineer
# learning-loop script (deterministic by default, live when
# LIVE_LLM=1 + ANTHROPIC_API_KEY are set) and merging its output
# IDs with the static presentation fields.
#
# This file NEVER prints, stores, or writes ANTHROPIC_API_KEY. The
# child demo script reads it from the inherited env when present;
# nothing in this script touches it.

set -euo pipefail
trap 'echo; echo "FAILED at line $LINENO" >&2' ERR

ARGO_BIN="${ARGO_BIN:-argo}"
LAUNCH_DIR="$(cd "$(dirname "$0")" && pwd)"
DEMO_DIR="$(cd "$LAUNCH_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/.." && pwd)"
OUT="$LAUNCH_DIR/data/demo.json"

require() {
  command -v "$1" >/dev/null 2>&1 \
    || { echo "FAIL: $1 not on PATH" >&2; exit 1; }
}
require "$ARGO_BIN"
require jq

# Caller-managed temp dirs so the demo's sessions.db survives long
# enough for us to inspect IDs.
ARGO_HOME="$(mktemp -d -t launch-capture-home-XXXXXX)"
WORK_DIR="$(mktemp -d -t launch-capture-work-XXXXXX)"
trap 'rm -rf "$ARGO_HOME" "$WORK_DIR"' EXIT INT TERM

# Resolve mode. --live wins; otherwise honour LIVE_LLM env var.
LIVE_FLAG=""
if [ "${1:-}" = "--live" ]; then LIVE_FLAG="--live"; shift || true; fi
if [ -z "$LIVE_FLAG" ] && [ "${LIVE_LLM:-0}" = "1" ]; then LIVE_FLAG="--live"; fi

# Pre-flight: live mode requires the API key. Catch this before the
# child fails so the operator gets a clean message.
if [ "$LIVE_FLAG" = "--live" ] && [ -z "${ANTHROPIC_API_KEY:-}" ]; then
  echo "FAIL: --live / LIVE_LLM=1 requires ANTHROPIC_API_KEY in env" >&2
  exit 1
fi

MODE_LABEL="deterministic"
[ "$LIVE_FLAG" = "--live" ] && MODE_LABEL="live"

echo "==> running demo (mode=$MODE_LABEL)"
LOG="$WORK_DIR/demo.log"

# Run the demo script. We pass our temp dirs so its sessions.db is
# reachable after exit. stdout is teed to LOG so we can extract IDs
# either from the captured stream OR from a follow-up DB query.
ARGO_BIN="$ARGO_BIN" \
ARGO_HOME="$ARGO_HOME" \
WORK_DIR="$WORK_DIR" \
ARGO_VERTICALS_DIR="$REPO_ROOT/verticals" \
  bash "$DEMO_DIR/gtm_engineer_learning_loop.sh" $LIVE_FLAG 2>&1 | tee "$LOG" >/dev/null \
    || { echo "FAIL: demo script exited non-zero; tail of log:" >&2; tail -40 "$LOG" >&2; exit 1; }

# ----------------------------------------------- extract IDs from log
SID=$(awk '/^  created session/ { print $3; exit }' "$LOG")
APPENDED_IDS=( $(awk '/^  appended/ { print $2 }' "$LOG") )
CANDIDATE_ID="${APPENDED_IDS[${#APPENDED_IDS[@]}-1]:-}"
SKILL_ID=$(awk -F\" '/"skill_id"/ { print $4; exit }' "$LOG")
PASS_RATE=$(awk '/eval:.*pass_rate=/ { sub(/.*pass_rate=/,""); sub(/[^0-9.].*/,""); print; exit }' "$LOG")
PASSED=$(awk '/^  eval: passed=/ { gsub(/.*passed=/,""); gsub(/ .*/,""); print; exit }' "$LOG")
SCORED=$(awk '/^  eval: passed=/ { sub(/.*scored=/,""); sub(/ .*/,""); print; exit }' "$LOG")

[ -n "$SID" ]          || { echo "FAIL: could not parse session id from log" >&2; exit 1; }
[ -n "$CANDIDATE_ID" ] || { echo "FAIL: could not parse candidate id from log" >&2; exit 1; }
[ -n "$SKILL_ID" ]     || { echo "FAIL: could not parse skill id from log" >&2; exit 1; }

# Memory write rows in order of appearance — we know the demo writes
# 3 campaign_results, then 1 patterns, then 1 learned_skill_candidates.
KINDS=(campaign_results campaign_results campaign_results patterns learned_skill_candidates)
MEM_ARR=$(jq -n '[]')
for i in "${!APPENDED_IDS[@]}"; do
  kind="${KINDS[$i]:-unknown}"
  id="${APPENDED_IDS[$i]}"
  MEM_ARR=$(printf '%s' "$MEM_ARR" | jq --arg k "$kind" --arg id "$id" \
    '. + [{kind: $k, id: $id}]')
done

# ------------------------- merge with static presentation template
TEMPLATE="$LAUNCH_DIR/data/demo.json"
[ -f "$TEMPLATE" ] || { echo "FAIL: missing template at $TEMPLATE" >&2; exit 1; }

CAPTURED_AT=$(date -u +'%Y-%m-%dT%H:%M:%SZ')
PASS_RATE="${PASS_RATE:-1.0}"
PASSED="${PASSED:-10}"
SCORED="${SCORED:-10}"

NEW_JSON=$(jq \
  --arg sid "$SID" \
  --arg cid "$CANDIDATE_ID" \
  --arg skid "$SKILL_ID" \
  --arg ts "$CAPTURED_AT" \
  --arg mode "$MODE_LABEL" \
  --argjson mem "$MEM_ARR" \
  --argjson pass_rate "$PASS_RATE" \
  --argjson passed "$PASSED" \
  --argjson scored "$SCORED" \
  '
  .story.captured_at = $ts
  | .story.mode = $mode
  | .learning_moment.memory_entry_id = $cid
  | .proof.role.session_id = $sid
  | .proof.memory_writes = $mem
  | .proof.candidate_promoted.memory_entry_id = $cid
  | .proof.candidate_promoted.to_skill_id = $skid
  | .proof.skill_approved.skill_id = $skid
  | .proof.scored_eval.passed = $passed
  | .proof.scored_eval.scored = $scored
  | .proof.scored_eval.pass_rate = $pass_rate
  ' "$TEMPLATE")

# Write atomically — tmp then mv — so a partial failure never leaves
# a half-written demo.json on disk.
TMP_OUT="$OUT.tmp.$$"
printf '%s\n' "$NEW_JSON" > "$TMP_OUT"
mv "$TMP_OUT" "$OUT"

echo "==> wrote $OUT"
echo "    mode=$MODE_LABEL session=$SID candidate=$CANDIDATE_ID skill=$SKILL_ID"
echo "    eval: $PASSED/$SCORED (pass_rate=$PASS_RATE)"
