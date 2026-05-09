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

  # Graceful pause: idle-sleep without releasing claims. Distinct from
  # /tmp/argo-stop (kill switch). Useful when the operator wants to halt
  # new claims temporarily — e.g., before a manual merge sweep, during a
  # ledger refactor, or while fixing the swarm tooling. Resume via
  # `rm /tmp/argo-pause`.
  if [ -f /tmp/argo-pause ]; then
    echo "[$(date +%H:%M:%S)] /tmp/argo-pause sentinel found. Idle-sleeping 60s. (rm to resume)"
    sleep 60
    continue
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

  # Pick the first claimable TODO row whose may-touch ceiling check passes.
  # Iterate candidates in ledger order; skip any refused by
  # check-pre-claim-ceiling.sh (deterministic refusal — retrying won't help,
  # but next candidate might be fine). Owner column may be claude or codex.
  CEILING_CHECK="$REPO/scripts/check-pre-claim-ceiling.sh"
  BRIEF_ID=""
  while IFS= read -r candidate; do
    if [ -z "$candidate" ]; then continue; fi
    if [ -x "$CEILING_CHECK" ] && ! "$CEILING_CHECK" "$candidate" >/dev/null 2>&1; then
      echo "[$(date +%H:%M:%S)] Skipping $candidate — pre-claim ceiling refusal (may-touch file at-risk of 500-LOC ceiling). Brief stays TODO; will become claimable after a split-prep lands."
      continue
    fi
    BRIEF_ID="$candidate"
    break
  done < <(awk -F'|' '
    /^\| .* \| (claude|codex) \| TODO \|/ {
      gsub(/^[ \t]+|[ \t]+$/, "", $2)
      print $2
    }' docs/build-plan/BUILD_STATUS.md)

  if [ -z "$BRIEF_ID" ]; then
    echo "[$(date +%H:%M:%S)] No claimable TODO row (or all remaining refused by ceiling check). Sleeping 120s."
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
      echo "[$(date +%H:%M:%S)] Likely causes (in order of frequency):"
      echo "    - working tree dirty in $REPO (conductor mid-edit; check /tmp/argo-pause)"
      echo "    - real race-loss to another tab (claim push 3× failed non-fast-forward)"
      echo "    - phantom-mod rescue couldn't recover (real uncommitted edits present)"
      echo "    - ledger row format the awk picks but claim-with-retry's regex rejects"
      echo "[$(date +%H:%M:%S)] Inspect: grep '$BRIEF_ID' $REPO/docs/build-plan/BUILD_STATUS.md"
      echo "[$(date +%H:%M:%S)] Working tree state:"
      git -C "$REPO" status --short 2>&1 | head -10
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

  # Per-iteration log retention. Each session's stream-json is teed to a
  # timestamped NDJSON in ~/.argo/worker-logs/ for post-mortem search
  # (e.g., "what did the worker actually do during the 5h-quota window?").
  # Files are tiny (~tens of KB per iteration) and human-greppable.
  LOG_DIR="${HOME}/.argo/worker-logs"
  mkdir -p "$LOG_DIR"
  LOG_FILE="$LOG_DIR/$(date -u +%Y%m%dT%H%M%SZ)-$BRIEF_ID.ndjson"

  # Worker heartbeat. The main-loop bash process touches a pulse file
  # every 30s during the iteration. If the iteration wedges (e.g., hung
  # cargo test, network partition, machine sleep), the pulse goes stale
  # and the conductor's heartbeat-sweep can detect the wedged worker
  # without waiting for the >2h-since-push proxy. See
  # scripts/check-worker-heartbeats.sh for the conductor-side reader.
  PULSE_DIR="/tmp/argo-worker-pulse"
  mkdir -p "$PULSE_DIR"
  PULSE_FILE="$PULSE_DIR/$BRIEF_ID"
  touch "$PULSE_FILE"
  ( while [ -f "$PULSE_FILE" ]; do
      touch "$PULSE_FILE" 2>/dev/null
      sleep 30
    done ) &
  PULSE_PID=$!

  echo "[$(date +%H:%M:%S)] Spawning fresh Claude session for $BRIEF_ID..."
  echo "[$(date +%H:%M:%S)] Log: $LOG_FILE"
  echo "[$(date +%H:%M:%S)] Pulse: $PULSE_FILE (PID $PULSE_PID)"
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
     tests for crates in the may-touch list (~30s-2min each).
  4b. Run cross-crate drift check before pushing:
        bash scripts/workspace-clippy-fast-fail.sh
      Cold first run ~9min; warm after that. With codex offline this
      is the only workspace-clippy gate; per-crate clippy alone misses
      cross-crate drift (struct fields, exhaustive matches, trait
      signature changes). Run in background while drafting PR body.
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
" 2>&1 | tee "$LOG_FILE" | jq -rR --unbuffered '
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
  | tee >(grep --line-buffered -oE "You.ve hit your (limit|usage).+" \
          | head -1 > /tmp/argo-quota-hit) \
  || echo "[$(date +%H:%M:%S)] Claude session exited non-zero for $BRIEF_ID. Conductor will see the stale state on next tick."

  # Stop the heartbeat: clear the pulse file (background loop drops on next
  # check) and explicitly kill the writer in case it's mid-sleep.
  rm -f "$PULSE_FILE"
  kill "$PULSE_PID" 2>/dev/null || true

  # Auto-resume on Anthropic 5h-quota exhaustion. The session prints e.g.
  # "You've hit your limit · resets 8:40pm (Australia/Melbourne)" and exits ~2s
  # later. Without this guard the bash loop happily claims the next brief, hits
  # quota again, and burns TODO rows into permanently-CLAIMED state. We:
  #   1. Detect the quota line (sentinel /tmp/argo-quota-hit, written by tee>grep above)
  #   2. Parse the reset time → seconds-to-sleep
  #   3. Release the just-claimed row back to TODO with a salvage note
  #   4. Sleep until quota reset (+5min buffer), then `continue` the outer loop.
  if [ -f /tmp/argo-quota-hit ] && [ -s /tmp/argo-quota-hit ]; then
    RESET_LINE=$(cat /tmp/argo-quota-hit)
    echo "[$(date +%H:%M:%S)] Anthropic quota exhausted. $RESET_LINE"

    # Compute seconds until reset; falls back to 1h on any parse failure.
    SLEEP_SECS=$(python3 - <<'PY'
import re, datetime, zoneinfo
try:
    line = open("/tmp/argo-quota-hit").read().strip()
    m = re.search(r"resets\s+(\d+(?::\d+)?)\s*([ap]m)\s*\(([^)]+)\)", line, re.I)
    if not m:
        print(3600)
    else:
        hhmm, ampm, tz = m.groups()
        hh, mm = (hhmm.split(":") + ["0"])[:2]
        hh = (int(hh) % 12) + (12 if ampm.lower() == "pm" else 0)
        zi = zoneinfo.ZoneInfo(tz)
        now = datetime.datetime.now(zi)
        target = now.replace(hour=hh, minute=int(mm), second=0, microsecond=0)
        if target <= now:
            target += datetime.timedelta(days=1)
        secs = int((target - now).total_seconds()) + 300
        print(max(secs, 60))
except Exception:
    print(3600)
PY
)

    # Release the just-claimed row → TODO so a fresh worker (this loop, post-sleep)
    # can re-claim. Salvage note points at /tmp/<brief-id> for any partial WIP.
    # Note: regex captures up-to-but-not-including the trailing ` |` so the
    # release note can be appended INSIDE the last cell, not after it.
    # (Prior version had `(.*)$` which swallowed the trailing pipe and
    # placed the note after it, breaking row format and tripping
    # claim-with-retry's regex on next iteration.)
    set +e
    python3 - "$BRIEF_ID" "$REPO" <<'PY'
import re, sys, pathlib
brief, repo = sys.argv[1:3]
p = pathlib.Path(repo) / "docs/build-plan/BUILD_STATUS.md"
text = p.read_text()
note = (f" **RELEASED by worker-loop quota guard; auto-resume pending:** "
        f"Anthropic 5h quota hit; session exited before opening PR. "
        f"/tmp/{brief} may have salvageable WIP.")
new, n = re.subn(
    rf"^(\| {re.escape(brief)} \| (?:claude|codex)) \| CLAIMED \|(.*) \|$",
    lambda m: f"{m.group(1)} | TODO |{m.group(2)}{note} |",
    text, count=1, flags=re.MULTILINE,
)
if n:
    p.write_text(new)
PY

    git -C "$REPO" add docs/build-plan/BUILD_STATUS.md
    if ! git -C "$REPO" diff --cached --quiet; then
      git -C "$REPO" commit -m "status: release $BRIEF_ID (quota; auto-resume pending)" 2>&1 | tail -1
      git -C "$REPO" pull --rebase origin main 2>&1 | tail -2
      git -C "$REPO" push origin main 2>&1 | tail -2
    else
      echo "[$(date +%H:%M:%S)] Row already non-CLAIMED; skipping release commit."
    fi
    set -e
    rm -f /tmp/argo-quota-hit

    # Resume time — best-effort across BSD (macOS) and GNU date.
    RESUME_AT=$(date -v+"${SLEEP_SECS}"S +%H:%M:%S 2>/dev/null \
             || date -d "@$(($(date +%s)+SLEEP_SECS))" +%H:%M:%S 2>/dev/null \
             || echo "after sleep")
    echo "[$(date +%H:%M:%S)] Sleeping ${SLEEP_SECS}s until quota reset (+5min buffer). Resume at ${RESUME_AT}."
    sleep "$SLEEP_SECS"
    echo "[$(date +%H:%M:%S)] Quota window expected open. Resuming loop."
    continue
  fi
  rm -f /tmp/argo-quota-hit  # belt-and-braces cleanup if file existed but was empty

  JITTER=$((RANDOM % 26 + 5))
  echo "[$(date +%H:%M:%S)] Iteration done. Jittering ${JITTER}s..."
  sleep "$JITTER"
done
