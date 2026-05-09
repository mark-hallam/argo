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

# Spin-loop guard: if the SAME brief fails to claim N times in a row, exit
# rather than burning CPU. claim-with-retry exit codes:
#   0 = claimed
#   1 = race lost / row already CLAIMED-by-another / BLOCKED → safe to pick a different row
#   2 = deterministic failure (brief not in ledger, or regex mismatch on the row)
# Same brief failing 2/2 means our awk + retry-script disagree on what's claimable —
# need an operator fix, not a tighter loop.
LAST_BRIEF=""
LAST_BRIEF_FAILS=0
MAX_SAME_BRIEF_FAILS=2

while true; do
  if [ -f /tmp/argo-stop ]; then
    echo "[$(date +%H:%M:%S)] /tmp/argo-stop sentinel found. Exiting."
    exit 0
  fi

  echo "[$(date +%H:%M:%S)] Sync main..."
  git checkout main
  # Defensive against stale multi-line FETCH_HEAD from a prior interrupted
  # fetch (which causes `git pull --ff-only` to fail with
  # "Cannot fast-forward to multiple branches"). Fetch + ff-merge explicit.
  if ! git pull --ff-only origin main 2>&1; then
    echo "[$(date +%H:%M:%S)] git pull failed; falling back to explicit fetch+merge"
    git fetch origin main
    git merge --ff-only FETCH_HEAD
  fi

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
  set +e
  bash scripts/claim-with-retry.sh "$BRIEF_ID"
  CLAIM_RC=$?
  set -e

  if [ "$CLAIM_RC" -ne 0 ]; then
    # Track consecutive failures on the same brief
    if [ "$BRIEF_ID" = "$LAST_BRIEF" ]; then
      LAST_BRIEF_FAILS=$((LAST_BRIEF_FAILS + 1))
    else
      LAST_BRIEF=$BRIEF_ID
      LAST_BRIEF_FAILS=1
    fi

    echo "[$(date +%H:%M:%S)] Claim failed (rc=$CLAIM_RC) for $BRIEF_ID — attempt ${LAST_BRIEF_FAILS}/${MAX_SAME_BRIEF_FAILS}."

    if [ "$LAST_BRIEF_FAILS" -ge "$MAX_SAME_BRIEF_FAILS" ]; then
      echo "[$(date +%H:%M:%S)] Same brief failed ${MAX_SAME_BRIEF_FAILS}× in a row. Exiting to prevent spin-loop."
      echo "[$(date +%H:%M:%S)] Likely root cause: ledger row format the awk picks but claim-with-retry's regex rejects."
      echo "[$(date +%H:%M:%S)] Inspect: grep '$BRIEF_ID' $REPO/docs/build-plan/BUILD_STATUS.md"
      exit 1
    fi

    # Back off so we don't burn CPU even when not yet at exit threshold
    echo "[$(date +%H:%M:%S)] Backing off 30s before retry."
    sleep 30
    continue
  fi

  # Successful claim resets the failure tracker
  LAST_BRIEF=""
  LAST_BRIEF_FAILS=0

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
     **Use rebase, NOT stash, for the post-edit push:**
       git pull --rebase origin main   # races land cleanly
       git add docs/build-plan/BUILD_STATUS.md
       git commit -m \"status: $BRIEF_ID in review (#<PR>)\"
       git push origin main
     If push is rejected (non-fast-forward): re-run \`git pull --rebase\`
     and retry. **Never** use \`git stash push\`/\`pop\` around this:
     stash-pop on already-pulled state leaves phantom \`M\` markers
     in the shared main checkout that block subsequent claim
     attempts (claim-with-retry has a rescue for this, but do not
     rely on it).

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
