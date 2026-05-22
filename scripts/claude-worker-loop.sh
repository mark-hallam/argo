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

# --- HARD GUARD: refuse to run if ANTHROPIC_API_KEY is in the environment ---
# Workers MUST use Claude Code subscription OAuth (the account auth) — never
# an API key. If ANTHROPIC_API_KEY is set, `claude -p` silently routes through
# API-key billing and burns paid credits instead of consuming Max-plan quota.
# That mode caused the 2026-05-10 credit-burn incident (workers spawning under
# a depleted org-credit account when subscription auth would have worked).
#
# This guard refuses to start when the var is non-empty, regardless of source
# (~/.zshenv, ~/.zshrc, parent shell, etc). Fix the env, not the script.
if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
  echo "" >&2
  echo "╔═══════════════════════════════════════════════════════════════════╗" >&2
  echo "║  🛑  worker-loop refused: ANTHROPIC_API_KEY is set                  ║" >&2
  echo "║                                                                     ║" >&2
  echo "║  Workers must use Claude Code subscription OAuth (account auth),    ║" >&2
  echo "║  never an API key. API-key billing burns paid credits instead of    ║" >&2
  echo "║  consuming Max-plan quota.                                          ║" >&2
  echo "║                                                                     ║" >&2
  echo "║  Fix: comment out ANTHROPIC_API_KEY in ~/.zshenv (or wherever it's  ║" >&2
  echo "║  exported), then open a fresh terminal and re-run this script.      ║" >&2
  echo "╚═══════════════════════════════════════════════════════════════════╝" >&2
  echo "" >&2
  exit 2
fi

REPO="${REPO:-~/code/argo}"
FAMILY="${FAMILY:-claude}"

# Per-bash-process sentinel for quota detection. Using a shared
# /tmp/argo-quota-hit causes a race when multiple worker tabs run:
# each tab's `tee >(grep ... > sentinel)` truncates the file at session
# start, which can clobber another tab's just-written quota signal
# before its auto-resume guard reads it. Per-PID isolation fixes that.
QUOTA_HIT="/tmp/argo-quota-hit-$$"

cd "$REPO"

# --- Clean-stop trap (SIGHUP/SIGTERM/SIGINT) -------------------------------
# If the operator closes the tab (SIGHUP), kills the loop (SIGTERM), or
# Ctrl-C's it (SIGINT) while a brief is CLAIMED, release the claim back to
# TODO so the swarm doesn't leave a zombie row. Best-effort — if the push
# races, the conductor's >2h stale-sweep will catch it.
#
# BRIEF_ID and PULSE_FILE are set inside the iteration. If the trap fires
# before they're set, the conditionals below silently no-op.
release_on_signal() {
  local sig="${1:-EXIT}"
  echo "" >&2
  echo "[$(date +%H:%M:%S)] === worker-loop received $sig — clean shutdown ===" >&2
  if [ -n "${BRIEF_ID:-}" ] && [ -n "${PULSE_FILE:-}" ] && [ -f "$PULSE_FILE" ]; then
    echo "[$(date +%H:%M:%S)] Releasing claim on $BRIEF_ID (if still CLAIMED)..." >&2
    rm -f "$PULSE_FILE"
    [ -n "${PULSE_PID:-}" ] && kill "$PULSE_PID" 2>/dev/null || true
    # Best-effort claim release. Skip if not still CLAIMED (worker may have
    # opened a PR + pushed IN_REVIEW already).
    set +e
    git -C "$REPO" fetch origin main 2>/dev/null
    git -C "$REPO" checkout main 2>/dev/null
    git -C "$REPO" pull --ff-only origin main 2>/dev/null
    if grep -qE "^\| ${BRIEF_ID} \| (claude|codex) \| CLAIMED \|" \
         "$REPO/docs/build-plan/BUILD_STATUS.md" 2>/dev/null; then
      python3 - "$BRIEF_ID" "$REPO" <<'PY' 2>/dev/null
import re, sys, pathlib
brief, repo = sys.argv[1:3]
p = pathlib.Path(repo) / "docs/build-plan/BUILD_STATUS.md"
text = p.read_text()
note = (f" **RELEASED by worker-loop signal-trap (SIGHUP/SIGTERM/SIGINT):** "
        f"tab closed mid-iteration. /tmp/{brief} may have salvageable WIP.")
new, n = re.subn(
    rf"^(\| {re.escape(brief)} \| (?:claude|codex)) \| CLAIMED \|(.*) \|$",
    lambda m: f"{m.group(1)} | TODO |{m.group(2)}{note} |",
    text, count=1, flags=re.MULTILINE,
)
if n:
    p.write_text(new)
PY
      git -C "$REPO" add docs/build-plan/BUILD_STATUS.md 2>/dev/null
      if ! git -C "$REPO" diff --cached --quiet 2>/dev/null; then
        git -C "$REPO" commit -m "status: release $BRIEF_ID (worker-loop signal trap)" 2>&1 | tail -1 >&2
        git -C "$REPO" pull --rebase origin main 2>&1 | tail -1 >&2
        git -C "$REPO" push origin main 2>&1 | tail -1 >&2
      fi
    fi
    set -e
  fi
  rm -f "$QUOTA_HIT" 2>/dev/null
  echo "[$(date +%H:%M:%S)] Exiting cleanly." >&2
  exit 0
}
trap 'release_on_signal HUP'  HUP
trap 'release_on_signal TERM' TERM
trap 'release_on_signal INT'  INT

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
  #
  # Backgrounded inside `{ ... ; } &` so we can capture the subshell PID and
  # spawn a zombie-detection watchdog. The watchdog distinguishes a
  # legitimate long iteration (cargo test mid-stream, ScheduleWakeup sleep)
  # from a true zombie (claude already emitted `result:success/error` but
  # never EOF'd stdout, so bash is wedged on the pipeline). See watchdog
  # block below. The `|| echo "Claude session exited non-zero..."` fallback
  # is preserved via the `if ! wait` after the pipeline closes.
  { claude --dangerously-skip-permissions \
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
  - Exit cleanly (run cleanup in step 9 below first).

9. **Disk cleanup before exit (MANDATORY — scoped to YOUR brief only).**
   Other workers (Claude lane A + B + codex) are running concurrently
   on this machine. You MUST only touch paths containing your \$BRIEF_ID.

   ALLOWED (scoped to your brief):
   - /tmp/\$BRIEF_ID/                       (your worktree)
   - /tmp/\$BRIEF_ID-target/                (your target dir)
   - /tmp/argo-shared-target-\$BRIEF_ID/    (alt target dir naming)

   FORBIDDEN (would damage other workers):
   - rm -rf /tmp/*-target                   (nukes every worker)
   - rm -rf /tmp/*                          (nukes everything)
   - cargo clean --workspace                (invalidates shared cache)
   - rm -rf ~/.cargo/registry               (cross-iteration cache)
   - rm -rf /tmp/argo-shared-target         (without brief-id suffix)
   - git worktree prune --expire=now        (too aggressive)

   Run from \$REPO (NOT inside the worktree):

   if [ -z \"\${BRIEF_ID:-}\" ]; then
     echo \"BRIEF_ID empty; skipping cleanup\" >&2
   else
     cd \$REPO
     git worktree remove --force \"/tmp/\$BRIEF_ID\" 2>/dev/null || true
     rm -rf \"/tmp/\$BRIEF_ID-target\" \"/tmp/argo-shared-target-\$BRIEF_ID\" 2>/dev/null || true
     git worktree prune
   fi

   Hard rules:
   - Scope is sacred: only paths containing \$BRIEF_ID may be touched. No globs.
   - Always 'cd \$REPO' BEFORE 'git worktree remove' (git refuses if cwd is
     inside the target worktree).
   - Use 'git worktree remove', NEVER 'rm -rf' the worktree directly.
   - NEVER touch ~/.cargo/registry/.
   - 'git worktree prune' is SAFE — it only removes refs to dirs that no
     longer exist on disk; it cannot touch another worker's active worktree.
   - If 'git worktree remove' fails, the PR didn't push cleanly. Don't
     force; investigate.

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
    else empty end'; } &
  PIPELINE_PID=$!

  # Try to locate the claude PID among the pipeline subshell's children.
  # `pgrep -P $PIPELINE_PID` lists direct children (claude, tee, jq); we
  # match comm to filter to the claude process. Try for up to ~5s while the
  # subshell spawns its pipeline; if we never find it, fall back to killing
  # the pipeline subshell itself in the watchdog.
  CLAUDE_PID=""
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    for child in $(pgrep -P "$PIPELINE_PID" 2>/dev/null || true); do
      comm=$(ps -o comm= -p "$child" 2>/dev/null | tr -d ' ')
      case "$comm" in
        *claude*) CLAUDE_PID="$child"; break 2 ;;
      esac
    done
    sleep 0.5
  done
  echo "[$(date +%H:%M:%S)] Pipeline PID=$PIPELINE_PID, claude PID=${CLAUDE_PID:-unresolved}"

  # ----------------------------------------------------------------------
  # Zombie-detection watchdog (claude-worker-loop-zombie-detection brief).
  #
  # Source: 4 observed zombie incidents (tick-38 through tick-45) where
  # claude emitted `{"type":"result","subtype":"success"}`, then wedged in
  # SN state without EOF-ing stdout. The bash `tee | jq` pipeline never
  # closed, so the outer loop never reached the post-iteration block.
  #
  # The watchdog fires ONLY when ALL FOUR conditions hold:
  #   1. The claude pipeline is still running (loop guard).
  #   2. $LOG_FILE mtime stale ≥20min (no streamed event in 20min).
  #   3. The last log line is a `result:success` or `result:error`
  #      event — i.e., claude already emitted its terminal event.
  #   4. $PULSE_FILE mtime stale ≥120s — the bash-side heartbeat has
  #      also stopped, confirming the loop is wedged on the pipeline.
  #
  # Operator-locked: NO hard per-session timeout. Legitimate long
  # iterations (cold workspace-clippy ~9min, 600-LOC briefs with
  # multi-crate tests 30-60min, ScheduleWakeup-based sleeps) must run
  # as long as they need. The 4-condition gate distinguishes zombie
  # from busy: a worker mid-cargo-test still streams tool_use events
  # (log mtime fresh); a worker mid-ScheduleWakeup still touches the
  # pulse from the bash side. Only true zombies satisfy all four.
  #
  # On trigger: SIGTERM the claude PID (or pipeline PID if claude
  # unresolved); escalate to SIGKILL after 30s grace.
  # ----------------------------------------------------------------------
  {
    while kill -0 "$PIPELINE_PID" 2>/dev/null; do
      sleep 60
      kill -0 "$PIPELINE_PID" 2>/dev/null || break  # condition 1

      now=$(date +%s)

      # condition 2: $LOG_FILE mtime stale ≥1200s (20min)
      log_mtime=$(stat -f %m "$LOG_FILE" 2>/dev/null || stat -c %Y "$LOG_FILE" 2>/dev/null || echo 0)
      log_age=$((now - log_mtime))
      [ "$log_age" -lt 1200 ] && continue

      # condition 3: last log event is result:success or result:error.
      # JSON field order is not stable — check both orderings.
      last_line=$(tail -1 "$LOG_FILE" 2>/dev/null || true)
      case "$last_line" in
        *'"type":"result"'*'"subtype":"success"'*) ;;
        *'"type":"result"'*'"subtype":"error"'*)   ;;
        *'"subtype":"success"'*'"type":"result"'*) ;;
        *'"subtype":"error"'*'"type":"result"'*)   ;;
        *) continue ;;
      esac

      # condition 4: $PULSE_FILE mtime stale ≥120s
      pulse_age=0
      if [ -f "$PULSE_FILE" ]; then
        pulse_mtime=$(stat -f %m "$PULSE_FILE" 2>/dev/null || stat -c %Y "$PULSE_FILE" 2>/dev/null || echo 0)
        pulse_age=$((now - pulse_mtime))
      fi
      [ "$pulse_age" -lt 120 ] && continue

      # ALL FOUR conditions hold → true zombie signature.
      echo "[$(date +%H:%M:%S)] 🧟 zombie-watchdog fired: claude=${CLAUDE_PID:-?} pipeline=$PIPELINE_PID log_age=${log_age}s pulse_age=${pulse_age}s last_event=result; sending SIGTERM..." >&2
      target_pid="${CLAUDE_PID:-$PIPELINE_PID}"
      kill -TERM "$target_pid" 2>/dev/null || true
      for _ in $(seq 30); do
        kill -0 "$target_pid" 2>/dev/null || break
        sleep 1
      done
      if kill -0 "$target_pid" 2>/dev/null; then
        echo "[$(date +%H:%M:%S)] 🧟 zombie-watchdog: SIGTERM ineffective after 30s; escalating SIGKILL." >&2
        kill -KILL "$target_pid" 2>/dev/null || true
      fi
      break
    done
  } &
  WATCHDOG_PID=$!

  # Wait for the pipeline to complete. Preserves the original `|| echo "..."`
  # semantics: if the pipeline exits non-zero (claude crash, jq parse failure,
  # SIGTERM/SIGKILL from the watchdog), print the friendly note. `if !`
  # avoids tripping `set -e`.
  if ! wait "$PIPELINE_PID"; then
    echo "[$(date +%H:%M:%S)] Claude session exited non-zero for $BRIEF_ID. Conductor will see the stale state on next tick."
  fi

  # Stop the watchdog (it may still be inside its 60s sleep; that's fine —
  # `kill` interrupts the sleep). Reap to suppress the "Done" job message.
  kill "$WATCHDOG_PID" 2>/dev/null || true
  wait "$WATCHDOG_PID" 2>/dev/null || true

  # Stop the heartbeat: clear the pulse file (background loop drops on next
  # check) and explicitly kill the writer in case it's mid-sleep.
  rm -f "$PULSE_FILE"
  kill "$PULSE_PID" 2>/dev/null || true

  # Quota detection — race-free. Previously this used `tee >(grep ... > sentinel)`
  # process substitution, which bash does NOT wait for; the [ -s "$QUOTA_HIT" ]
  # check ran before grep flushed its match, so when claude exited with
  # `result: success` (which it does when the limit message is rendered as a
  # successful assistant turn), the auto-resume guard silently fell through and
  # the next iteration also hit quota. Result: tabs that hit quota a second
  # time never auto-recovered.
  #
  # Fix: $LOG_FILE is the raw stream-json, fully synced to disk by the inline
  # `tee "$LOG_FILE"` above. Grep it AFTER the pipeline closes — no race.
  RESET_LINE=$(grep -oE "You.ve hit your (limit|usage)[^\\\"]+" "$LOG_FILE" 2>/dev/null | head -1 || true)
  if [ -n "$RESET_LINE" ]; then
    echo "$RESET_LINE" > "$QUOTA_HIT"
  fi

  # Auto-resume on Anthropic 5h-quota exhaustion. The session prints e.g.
  # "You've hit your limit · resets 8:40pm (Australia/Melbourne)" and exits ~2s
  # later. Without this guard the bash loop happily claims the next brief, hits
  # quota again, and burns TODO rows into permanently-CLAIMED state. We:
  #   1. Detect the quota line (post-pipeline grep of $LOG_FILE)
  #   2. Parse the reset time → seconds-to-sleep
  #   3. Release the just-claimed row back to TODO with a salvage note
  #   4. Sleep until quota reset (+5min buffer), then `continue` the outer loop.
  if [ -f "$QUOTA_HIT" ] && [ -s "$QUOTA_HIT" ]; then
    RESET_LINE=$(cat "$QUOTA_HIT")
    echo ""
    echo "╔═══════════════════════════════════════════════════════════════════╗"
    echo "║  🛑  ANTHROPIC QUOTA EXHAUSTED — $(date +%H:%M:%S)                       ║"
    echo "║  $RESET_LINE"
    echo "║  Tab will sleep until reset + 5min buffer, then auto-resume.       ║"
    echo "║  Heartbeat every 5min so you can see the tab is alive.             ║"
    echo "╚═══════════════════════════════════════════════════════════════════╝"
    echo ""

    # Compute seconds until reset; falls back to 1h on any parse failure.
    SLEEP_SECS=$(QUOTA_HIT="$QUOTA_HIT" python3 - <<'PY'
import os, re, datetime, zoneinfo
try:
    line = open(os.environ["QUOTA_HIT"]).read().strip()
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
    rm -f "$QUOTA_HIT"

    # Resume time — best-effort across BSD (macOS) and GNU date.
    RESUME_AT=$(date -v+"${SLEEP_SECS}"S +%H:%M:%S 2>/dev/null \
             || date -d "@$(($(date +%s)+SLEEP_SECS))" +%H:%M:%S 2>/dev/null \
             || echo "after sleep")
    echo "[$(date +%H:%M:%S)] Sleeping ${SLEEP_SECS}s until quota reset (+5min buffer). Resume at ${RESUME_AT}."
    # Heartbeat every 5 min during the long sleep so the operator sees that
    # the tab is alive and waiting (instead of looking like a dead terminal).
    # SIGHUP/TERM/INT trap above still applies — the tab can be Ctrl-C'd
    # cleanly during this sleep and the claim will be released.
    REMAINING=$SLEEP_SECS
    HEARTBEAT_INTERVAL=300  # 5 minutes
    while [ "$REMAINING" -gt 0 ]; do
      THIS_TICK=$REMAINING
      [ "$THIS_TICK" -gt "$HEARTBEAT_INTERVAL" ] && THIS_TICK=$HEARTBEAT_INTERVAL
      sleep "$THIS_TICK"
      REMAINING=$((REMAINING - THIS_TICK))
      if [ "$REMAINING" -gt 0 ]; then
        MINS=$((REMAINING / 60))
        echo "[$(date +%H:%M:%S)] 💤 quota-sleep — ${MINS}min remaining (resume ~${RESUME_AT})"
      fi
    done
    echo "[$(date +%H:%M:%S)] ✅ Quota window expected open. Resuming loop."
    continue
  fi
  rm -f "$QUOTA_HIT"  # belt-and-braces cleanup if file existed but was empty

  # ----------------------------------------------------------------------
  # Post-iteration "did claude actually ship?" check.
  #
  # The watchdog above kills true zombies (claude wedged after emitting
  # `result:success/error`), but a worker can also bail without shipping
  # for non-zombie reasons (network partition during gh pr create, OOM,
  # SIGKILL from outside, ScheduleWakeup wedge after compaction, etc.).
  # In every case the failure mode is the same: brief stays CLAIMED with
  # no PR opened, the conductor must orphan-sweep + demote.
  #
  # Mirrors the quota guard's release block: same regex shape, same
  # Python regex, same commit+rebase+push pattern. Triggered by a
  # different signal (still-CLAIMED on origin/main after pipeline close).
  # Runs AFTER the quota check so we don't double-release when quota
  # already released the row.
  # ----------------------------------------------------------------------
  set +e
  git -C "$REPO" fetch origin main 2>&1 | tail -1
  git -C "$REPO" pull --ff-only origin main 2>&1 | tail -1
  if grep -qE "^\| ${BRIEF_ID} \| (claude|codex) \| CLAIMED \|" \
       "$REPO/docs/build-plan/BUILD_STATUS.md" 2>/dev/null; then
    echo "[$(date +%H:%M:%S)] 🧟 ship-check: $BRIEF_ID still CLAIMED on main; releasing to TODO."
    python3 - "$BRIEF_ID" "$REPO" <<'PY'
import re, sys, pathlib
brief, repo = sys.argv[1:3]
p = pathlib.Path(repo) / "docs/build-plan/BUILD_STATUS.md"
text = p.read_text()
note = (f" **RELEASED by worker-loop zombie-detection:** "
        f"worker exited without opening PR; /tmp/{brief} may have salvageable WIP.")
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
      git -C "$REPO" commit -m "status: release $BRIEF_ID (zombie; no PR opened)" 2>&1 | tail -1
      git -C "$REPO" pull --rebase origin main 2>&1 | tail -2
      git -C "$REPO" push origin main 2>&1 | tail -2
    else
      echo "[$(date +%H:%M:%S)] Row already non-CLAIMED; skipping ship-check release commit."
    fi
  fi
  set -e

  # Defensive disk cleanup — SCOPED STRICTLY to this iteration's BRIEF_ID.
  # If the worker followed step 9 of its prompt the worktree is already
  # gone, but workers sometimes skip it under quota pressure or context
  # cap. This safety net repeats the same scoped cleanup at the bash
  # level so /tmp doesn't accumulate over a long-running loop.
  #
  # MULTI-WORKER SAFETY: other workers (Claude lane A + B + codex)
  # are running concurrently on this machine. Every path below contains
  # ${BRIEF_ID} so only THIS iteration's artifacts are touched. Never
  # glob /tmp/*-target; never touch ~/.cargo/registry; never use
  # 'git worktree prune --expire=now'. Plain 'git worktree prune' is
  # safe — it only removes refs to dirs that no longer exist on disk
  # (other workers' active worktrees stay intact).
  if [ -n "${BRIEF_ID:-}" ]; then
    cd "$REPO" 2>/dev/null || true

    # Sanity check: verify the worktree at /tmp/${BRIEF_ID} actually
    # belongs to us (branch name contains our brief id). If not,
    # skip the remove — could be a leftover from a different worker
    # or a same-named path collision.
    if [ -d "/tmp/${BRIEF_ID}/.git" ] || [ -d "/tmp/${BRIEF_ID}" ]; then
      ACTUAL_BRANCH=$(git -C "/tmp/${BRIEF_ID}" branch --show-current 2>/dev/null || echo "")
      case "$ACTUAL_BRANCH" in
        *"$BRIEF_ID"*)
          git worktree remove --force "/tmp/${BRIEF_ID}" 2>/dev/null || true
          ;;
        *)
          echo "[$(date +%H:%M:%S)] 🛡  worktree /tmp/${BRIEF_ID} branch='${ACTUAL_BRANCH}' doesn't match brief; skipping remove (multi-worker safety)" >&2
          ;;
      esac
    fi

    # Target dirs are unambiguously per-brief by naming convention.
    rm -rf "/tmp/${BRIEF_ID}-target" "/tmp/argo-shared-target-${BRIEF_ID}" 2>/dev/null || true

    # 'git worktree prune' only removes worktree refs whose dirs no
    # longer exist on disk. Other workers' active worktrees survive.
    git worktree prune 2>/dev/null || true

    # Report freed space periodically (every 10 iterations).
    ITER_COUNT=$((${ITER_COUNT:-0} + 1))
    if [ "$((ITER_COUNT % 10))" -eq 0 ]; then
      echo "[$(date +%H:%M:%S)] 🧹 /tmp disk usage after $ITER_COUNT iterations:"
      df -h /tmp 2>/dev/null | tail -1 || true
    fi
    export ITER_COUNT
  fi

  JITTER=$((RANDOM % 26 + 5))
  echo "[$(date +%H:%M:%S)] Iteration done. Jittering ${JITTER}s..."
  sleep "$JITTER"
done
