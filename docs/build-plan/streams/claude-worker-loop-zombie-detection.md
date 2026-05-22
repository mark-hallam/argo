# claude-worker-loop-zombie-detection

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `50fef9666` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-worker-loop-zombie-detection`.

> **Known gap — see follow-up brief [`claude-worker-loop-zombie-detection-followup`](claude-worker-loop-zombie-detection-followup.md).** The 4-condition gate landed by #2038 failed to fire on the PID-14468 incident (2026-05-22). Two defects: (1) condition #3 checked the LAST log line for `result:*`, but Claude Code's background-task wake emits `system:init` AFTER a result; when claude wedged on the wake, the last line was `init`, not result. (2) condition #4 (pulse-file mtime) is touched by a sidecar subshell independent of claude's wedge state, so it never staled. The followup brief patches condition #3 to grep the whole log for any result event, and replaces condition #4 with a claude-PID process-state check (`SN` = wedge). The conductor-side `check-zombie-workers.sh` aligns to the same heuristic.

🟡 **P2 OPS-TOOLING RELIABILITY.** Source: 4 zombie-worker incidents observed in tick-38 through tick-45 (PIDs 92458 / 17940 / 18871 / 59400). Each followed the same pattern: claude session emits `{"type":"result","subtype":"success"}` event, then wedges in `SN` (sleeping nice'd) state instead of fully exiting. The bash loop's pipeline (`claude | tee | jq`) never closes, so the outer loop never reaches its post-iteration step (quota check, disk cleanup, next claim). Result: a worker chain sits burning a PID slot, an active pulse file, a worktree on disk, and (worst) leaves the brief CLAIMED with no PR → the conductor must manually orphan-sweep + demote.

**Design constraint (operator-locked 2026-05-22):** **No hard per-session timeout.** A legitimate long-running iteration — cold workspace-clippy (~9min), a fresh 600-LOC brief with multi-crate tests (30-60min), a `ScheduleWakeup`-based wait pattern that genuinely sleeps for cargo to finish — must run as long as it needs. The fix targets the **zombie signature**, not session length.

## Scope

**1. Log-staleness watchdog (sidecar process, NOT a hard timeout).** Spawn a background watchdog alongside the claude invocation. It samples the iteration's `$LOG_FILE` mtime every 60s. The watchdog triggers ONLY when ALL of the following hold:
- The claude pipeline is still running (the outer loop hasn't yet reached the post-iteration block).
- The log file's mtime has been stale for ≥20 min (no jq output, meaning no completed assistant message, no tool call, no result event has flushed in 20 min).
- The log's LAST event line was `{"type":"result","subtype":"success"}` or `{"type":"result","subtype":"error"}` (i.e., claude already emitted its terminal event — true zombie signature, not mid-thought).
- The pulse file mtime is also stale (pulse is touched every 30s; if it's >120s old, the loop's heartbeat sidecar has stopped, which means bash is wedged on the pipeline, not paused mid-iteration).

When the watchdog fires, it sends `SIGTERM` to the claude process. If claude doesn't exit within 30s, escalates to `SIGKILL`. The outer bash loop then proceeds to the existing post-iteration block (quota check + new ship-check + cleanup + next claim).

**Critical:** the watchdog must NOT fire on a legitimately-long but ACTIVE session. The "last event was `result:success/error` AND log silent ≥20min AND pulse silent ≥2min" combination is what distinguishes zombie from busy. A worker mid-cargo-test still updates the log (tool_use events stream) and still touches the pulse. A worker mid-`ScheduleWakeup` still touches the pulse from the bash side.

**2. Post-iteration "did claude actually ship?" check.** Immediately after the claude pipeline closes (and the existing quota check passes), fetch `origin/main` fresh and check the brief's row. If the row is still CLAIMED (no PR opened, no BLOCKED note, no MERGED record), the worker bailed without shipping. Release the row to TODO with a salvage note ("worker exited without opening PR — possible zombie or self-aborted") and continue the loop. Mirrors the existing quota-release logic — same Python regex + commit + push — just triggered by a different signal.

**3. Sidecar detection script for the conductor.** `NEW: scripts/check-zombie-workers.sh` — runs `ps` to list all `claude --dangerously-skip-permissions` processes, parses each one's `(state, etime, brief, log_mtime, last_event)`, cross-checks the brief's row state on `origin/main`, and prints a colored report:
- 🟢 **healthy** — state R/R+ or S+, recent log activity (≤10min), brief CLAIMED with active pulse
- 🟡 **suspicious** — state SN, log silent 10-20min, brief still CLAIMED, last event was a tool_use or assistant message (might just be a long cargo run)
- 🔴 **zombie** — state SN, log silent ≥20min, last event was `result:success` or `result:error`, brief still CLAIMED (no PR)

The conductor reads this output during sweep ticks and decides which to kill. The script does NOT kill on its own — kill authority stays with the conductor.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `scripts/claude-worker-loop.sh` — narrowly scoped:
  - Add a watchdog background process spawned right before the claude invocation. It receives `$LOG_FILE`, `$PULSE_FILE`, and the claude PID (captured via `$!` after backgrounding the pipeline — needs a small refactor; see Notes). Implement the 4-condition gate above. On trigger, `kill -TERM <claude_pid>` then `kill -KILL` after 30s.
  - After the existing quota check block, add a sibling "did-claude-ship" check: `git fetch origin main && git pull --ff-only`, then `grep -E "^\| ${BRIEF_ID} \| (claude\|codex) \| CLAIMED \|"` — if matched, run the same Python release block the quota guard uses, with note ("worker exited without opening PR — possible zombie").
  - No changes to the quota guard itself, the heartbeat pulse, the signal trap, or the cleanup block.
  - **No `timeout` wrapper. No hard ceiling on session length.** Legitimate long iterations stay legitimate.
- `NEW: scripts/check-zombie-workers.sh` — the sidecar detection script. ~80-120 LOC bash.
- `NEW: scripts/check-zombie-workers-tests.sh` (or inline `# ---- tests ----` block + a sample-data fixture) — at least a smoke test that runs the script on an empty pool + on a synthetic ps listing.

## Files you must NOT touch

- The claude invocation prompt itself (the long `-p "You are a SKYCOT $FAMILY worker..."` block) — locked.
- The signal trap (`release_on_signal`) — out of scope.
- The quota guard's Python release block — reuse the regex shape, do NOT modify. Add a sibling block.
- `scripts/check-worker-heartbeats.sh` — existing pulse-file reader; out of scope.
- `scripts/claim-with-retry.sh` — claim path, not exit path.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] `scripts/claude-worker-loop.sh` spawns a watchdog process whose triggers ALL of: claude pipeline still running, `$LOG_FILE` mtime stale ≥20min, last log event is `result:success` or `result:error`, `$PULSE_FILE` mtime stale ≥120s. Verified by grep + reading the new block.
- [ ] The watchdog sends `SIGTERM` then `SIGKILL` after 30s grace.
- [ ] **No `timeout` command appears in `scripts/claude-worker-loop.sh`.** (Pre-existing `timeout` usages, if any, must be unchanged.)
- [ ] After the quota check, a new "did-claude-ship" check runs. If the brief is still CLAIMED on main, the row is released to TODO with the note `**RELEASED by worker-loop zombie-detection:** worker exited without opening PR; /tmp/<brief-id> may have salvageable WIP.` and committed + pushed.
- [ ] The release commit message format is `status: release <brief-id> (zombie; no PR opened)`.
- [ ] `scripts/check-zombie-workers.sh` exists, is executable, and produces tab-separated rows: `STATUS\tPID\tBRIEF\tSTATE\tETIME\tLOG_AGE_MIN\tLAST_EVENT`.
- [ ] Running `bash scripts/check-zombie-workers.sh` against a clean pool prints `(no claude workers alive)` and exits 0.
- [ ] Documentation comment block at the top of `check-zombie-workers.sh` explains the heuristic + the conductor's expected reaction to each color tier.
- [ ] **No regression**: an existing worker that ships a PR cleanly stays unaffected. An existing worker mid-cargo-test (legitimate long iteration with active log + active pulse) stays unaffected. Only sessions matching all four zombie criteria are touched.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-worker-loop-zombie-detection`
- PR title: `ops(worker-loop): log-staleness watchdog + post-iteration ship-check + zombie sidecar (claude-worker-loop-zombie-detection)`

## Notes for the implementer

- ~150-250 LOC total expected. Bash watchdog (~50), post-iteration ship-check block (~30), sidecar script (~80-100), tests (~30).
- **The zombie pattern in detail** (from forensic observation during tick-45):
  - claude finishes its work (PR opened, ledger flipped, worktree cleaned, all step-9 items done).
  - claude emits `{"type":"result","subtype":"success"}` to stdout.
  - The `tee | jq` pipeline downstream processes the line and writes to log.
  - claude does NOT exit. The process transitions to `S` state, then to `SN` (nice'd-down), and just sits there.
  - The bash loop is blocked on the pipeline closing (because claude hasn't EOF'd stdout).
  - Bash never reaches the post-iteration steps. Worker chain wedged.
  - Visible symptoms: claude pid in `SN` state, log untouched for many minutes, brief row may be IN_REVIEW (if PR shipped) or CLAIMED (if claude bailed before push).

- **Why the 4-condition gate is load-bearing**: a single signal (e.g., "log silent 20min") would false-positive on legitimate long iterations. The combination — log silent AND pulse silent AND last event was a terminal result — is the unique zombie signature. Workers in legitimate long sleeps still update the pulse (the bash sidecar's `touch` loop runs every 30s on the bash side). Workers mid-tool-use produce streaming events. Only zombies satisfy all four.

- **Backgrounding the claude pipeline**: currently the pipeline is `claude ... 2>&1 | tee "$LOG_FILE" | jq ...`. To get the claude PID, refactor to:
  ```bash
  claude ... 2>&1 | tee "$LOG_FILE" | jq ... &
  PIPELINE_PID=$!
  CLAUDE_PID=$(pgrep -P $PIPELINE_PID -f "claude --dangerously" | head -1)
  ```
  or capture via `coproc` or a temp pipefile — pick whichever fits cleanest in the existing flow. The current `|| echo "[..]" Claude session exited non-zero..."` fallback at the end of the pipeline still works.

- **Don't change the existing quota detection regex**. The `$QUOTA_HIT` sentinel is load-bearing; reuse but don't extend.

- **Why the sidecar detection script**: workers that started before this brief lands will still produce zombies that need manual cleanup. The sidecar gives the conductor a clean read instead of the ad-hoc `ps + ls + grep` dance from tick-45.

- **PR body must include a before/after diff of the claude invocation block** showing the exact watchdog spawn point + the post-iteration ship-check insertion point, so reviewers can verify nothing accidentally breaks the existing quota / cleanup flows.

- **Test the watchdog deliberately**: include a manual-test recipe in the PR body. Suggested: run worker loop against a brief, observe a healthy iteration completes; then artificially simulate a zombie (e.g., a test-only brief that writes a fake `result:success` line to a log + sleeps 30min) and confirm the watchdog fires. Operator can run this once after merge to gain trust.
