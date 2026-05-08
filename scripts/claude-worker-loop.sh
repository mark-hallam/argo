#!/usr/bin/env bash
# claude-worker-loop.sh — stateless Claude Code worker loop.
#
# Outer loop = bash (state machine: pull, claim, jitter).
# Inner = `claude -p` invocation (fresh context window per brief).
#
# Each iteration picks the next claimable TODO from BUILD_STATUS.md,
# claims it via scripts/claim-with-retry.sh, then hands off to a fresh
# headless Claude session that ships the PR for that one brief and
# exits. Bash loops to the next iteration.
#
# Workers running under this loop are NOT subject to the graceful-exit
# protocol (SWARM_PROTOCOL.md) — every iteration is a fresh window, so
# context never accumulates and there is no degraded-session mode.
#
# Usage:
#   bash scripts/claude-worker-loop.sh
#
# Environment:
#   REPO     — absolute path to the repo. Default: ~/code/argo
#   FAMILY   — claude (default) or codex (only if a `codex -p` equivalent
#              is wired; for now, claude only).
#
# Stop conditions:
#   - File /tmp/argo-stop exists.
#   - Operator Ctrl-C.

set -euo pipefail

REPO="${REPO:-~/code/argo}"
FAMILY="${FAMILY:-claude}"

cd "$REPO"

while true; do
  if [ -f /tmp/argo-stop ]; then
    echo "[$(date +%H:%M:%S)] /tmp/argo-stop sentinel found. Exiting."
    exit 0
  fi

  echo "[$(date +%H:%M:%S)] Sync main..."
  git checkout main
  git pull --ff-only origin main

  # Find the first claimable TODO row. Owner column may be claude or
  # codex; brief-id is the first table cell.
  BRIEF_ID=$(awk -F'|' '
    /^\| .* \| (claude|codex) \| TODO \|/ {
      gsub(/^[ \t]+|[ \t]+$/, "", $2)
      print $2
      exit
    }' docs/build-plan/BUILD_STATUS.md)

  if [ -z "$BRIEF_ID" ]; then
    echo "[$(date +%H:%M:%S)] No claimable TODO row. Sleeping 120s."
    sleep 120
    continue
  fi

  echo "[$(date +%H:%M:%S)] Claiming $BRIEF_ID..."
  if ! bash scripts/claim-with-retry.sh "$BRIEF_ID"; then
    echo "[$(date +%H:%M:%S)] Claim failed/lost for $BRIEF_ID. Restarting iteration."
    continue
  fi

  echo "[$(date +%H:%M:%S)] Spawning fresh Claude session for $BRIEF_ID..."
  # --dangerously-skip-permissions: required for unattended headless mode.
  # The bash wrapper holds the boundary (worktree isolation, conductor merges,
  # HARD RULE 5 may-touch list) so per-tool prompts are not the right defense.
  #
  # --output-format stream-json + jq filter: streams completed assistant
  # messages and tool calls to the terminal so the operator sees progress
  # instead of silence during long iterations. Default text format buffers
  # the full response until completion (the silence trap).
  #
  # No --include-partial-messages: those produce ~15 stream_event chunks
  # per real action — pure noise. We want completed events only.
  claude --dangerously-skip-permissions \
         --output-format stream-json \
         --verbose \
         -p "You are a SKYCOT $FAMILY worker.

Repo: $REPO
Brief: $BRIEF_ID (already CLAIMED on origin/main).

Required reading (in order):
  1. docs/build-plan/SWARM_PROTOCOL.md
  2. docs/build-plan/streams/$BRIEF_ID.md

Your job — ONE brief, then exit:
  1. git worktree add /tmp/$BRIEF_ID origin/main
  2. cd /tmp/$BRIEF_ID && git checkout -b $FAMILY-$BRIEF_ID
  3. Implement the brief inside its may-touch list.
  4. Run verification per SWARM_PROTOCOL.md — crate-scoped clippy +
     tests for crates in the may-touch list. Claude tabs are exempt
     from the workspace clippy step (codex-only via pre-push gate).
  5. Apply HARD RULE 8 pre-push flight check.
  6. git push -u origin $FAMILY-$BRIEF_ID
  7. gh pr create with title and body matching the brief.
  8. cd back to $REPO main checkout. Edit BUILD_STATUS.md row to
     IN_REVIEW with the PR number, commit, push to main.

If you cannot ship (premise wrong, may-touch too tight, 3-attempt fail):
  - Edit BUILD_STATUS.md row to BLOCKED with a one-line reason.
  - Commit and push to main.
  - Exit cleanly.

DO NOT loop. DO NOT claim another brief. The outer bash loop handles
the next iteration.
" 2>&1 | jq -rR --unbuffered '
    fromjson? // empty |
    if .type == "assistant" and .message.content then
      (.message.content[]? | (
        if .type == "text" then "  💬 " + ((.text // "") | gsub("\n"; " ") | .[0:200])
        elif .type == "tool_use" then "  🔧 " + .name + "  " + ((.input // {} | tostring) | .[0:120])
        else empty end
      ))
    elif .type == "result" then "  ✅ result: " + (.subtype // "ok")
    elif .type == "system" and .subtype == "init" then "  ▶️  session started"
    else empty end' \
  || echo "[$(date +%H:%M:%S)] Claude session exited non-zero for $BRIEF_ID. Conductor will see the stale state on next tick."

  JITTER=$((RANDOM % 26 + 5))
  echo "[$(date +%H:%M:%S)] Iteration done. Jittering ${JITTER}s..."
  sleep "$JITTER"
done
