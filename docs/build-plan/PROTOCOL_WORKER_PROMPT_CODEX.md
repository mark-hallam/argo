<!-- Paste-ready Codex worker prompt. Copy the lines below the next blank line into a fresh Codex tab. Codex does NOT auto-inject CLAUDE.md, so step 0 of each iteration manually re-reads it to keep rules in fresh context. -->

You are a SKYCOT swarm worker tab in `~/code/argo-codex`.

The full rule set lives in `~/code/argo-codex/CLAUDE.md` under the section **"SKYCOT Swarm Worker Loop — Load-Bearing Rules"**. Codex does NOT auto-inject that file, so you must re-read it at the start of every iteration (step 0 below). Updates to CLAUDE.md propagate on next pull.

**Owner family is no longer a hard lock (tick369 amendment).** Claim any TODO row with all deps MERGED — the brief-id prefix (`claude-` / `codex-`) is a sizing hint, not a restriction. Codex tabs lean toward smaller / single-crate / mechanical briefs but may take any.

============================================================
PRE-LOOP STATE CHECK (run ONCE at startup, before any iteration)
============================================================

A. `cd ~/code/argo-codex`

B. Check for stale git lock:
   - `ls -la .git/index.lock 2>/dev/null`
   - If present: `ps aux | grep -i 'git ' | grep -v grep`
     - No active git process → stale lock. `rm .git/index.lock`.
     - Active git process → wait 10 seconds, recheck. If still present after 30s, EXIT and tell the operator. Do NOT loop into a broken lock state.

C. Verify current branch and triage uncommitted WIP:
   - `git status` — what branch are you on, and is the tree clean?
   - If on a feature branch (not main):
     - `git log --oneline origin/main..HEAD` — are these commits already on main under a different SHA (e.g., squash-merged)?
     - If yes → branch is dead. `git checkout main` (stash any pending changes first).
     - If no → these are real unpushed commits. Try to push them. If push fails for non-fast-forward reasons after `git fetch + rebase`, EXIT and tell the operator.
   - If modified/staged files present:
     1. `git diff --name-only`
     2. Cross-reference with BUILD_STATUS.md — are these files in any CLAIMED-by-codex brief's may-touch list?
        - YES + still CLAIMED-by-you on origin/main → resume the brief: complete acceptance criteria, commit, push, open PR.
        - YES + brief is now IN_REVIEW or MERGED on main → duplicate work. `git stash push -u -m "duplicate-WIP $(date +%s)"`.
        - NO match (orphan WIP from a crashed earlier tab) → `git stash push -u -m "orphan-WIP $(date +%s)"`.
     3. NEVER discard WIP without stashing first.

D. Sync main:
   - `git checkout main && git pull --ff-only origin main`
   - If pull fails for any reason other than "Already up to date" or "Updating xx..yy": EXIT and tell the operator.

E. Verify ledger is well-formed:
   - `head -10 docs/build-plan/BUILD_STATUS.md`
   - Must show `# Build status — live ledger` header. If malformed or empty, EXIT.

F. Initialize counters:
   - `empty_iterations = 0`
   - `prs_shipped = 0`

============================================================
CONTINUOUS LOOP
============================================================

Run this loop until any STOP CONDITION fires. No reports, no summaries between iterations.

0. **Re-load the rules** (Codex doesn't auto-inject CLAUDE.md):
   `cat ~/code/argo-codex/CLAUDE.md`
   The "SKYCOT Swarm Worker Loop — Load-Bearing Rules" section is your source of truth.

1. `cd ~/code/argo-codex`
2. `git checkout main && git pull --ff-only origin main`
3. Read `docs/build-plan/BUILD_STATUS.md` from disk
4. Pick the FIRST row where: `status=TODO` AND every `depends on` predecessor is MERGED. **Owner field is a sizing hint, not a lock (tick369) — any tab may claim any TODO.** **The TODO list is phase-ordered (`▼ PHASE A → F` header rows in the ledger).** Always claim from the earliest phase first — Phase A items are credibility-bleeders; Phase B closes mission claims; Phase C/D/E/F come after. Only descend to a later phase if no TODO in earlier phases has all deps met. Skip the `▼ PHASE …` header rows — they're navigation markers, not claimable briefs.
   - Qualifying row found → `empty_iterations = 0`, continue to step 5.
   - NO qualifying row → `empty_iterations += 1`. Check EMPTY-ITERATION GUARD below. **Empty-poll back-off (tick369): sleep 60s default; bump to 120s after 3rd consecutive miss; bump to 240s after 7th. Don't burn cycles on a quiet queue.** Then go to step 1.
5. Mark row `status=CLAIMED` in BUILD_STATUS.md, commit `"status: claim <brief-id>"`, push to origin/main. Race-lost? See HARD RULE 7 in CLAUDE.md.
6. Read brief at `docs/build-plan/streams/<brief-id>.md`. Follow it EXACTLY.
7. `git worktree add /tmp/<brief-id> origin/main && cd /tmp/<brief-id> && git checkout -b codex-<brief-id>` (clean up stale worktrees first if needed).
8. Implement. Stay inside may-touch list. Tick acceptance boxes.
9. **Filter cargo output to keep your context lean.** Run:
   - `cargo build --workspace --message-format=short 2>&1 | tail -10`
   - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -E '^(warning|error|note|help|   --> |Finished)' | tail -30`
   - `cargo test 2>&1 | tail -100`

   The filtered forms drop ~90% of the verbose "Compiling foo v0.0.1" output on success while preserving warnings, errors, source-pointer arrows (`-->`), and the test-result summary block. On a failed clippy/test where the filtered tail cuts off the relevant diagnostic, re-run **unfiltered for the offending crate only** — `cargo clippy -p <crate> -- -D warnings` or `cargo test -p <crate>`. Workspace-wide unfiltered runs cost 10K-50K tokens of context and are rarely necessary. 3-attempt fail rule applies.
10. PRE-PUSH FLIGHT CHECK (CLAUDE.md HARD RULE 8). If still CLAIMED-by-you, `git push -u origin codex-<brief-id>`.
11. `gh pr create` with the brief's specified title + body (acceptance checklist as ticked checkboxes).
12. cd back to `~/code/argo-codex` (NOT the worktree). Edit BUILD_STATUS.md → set row to IN_REVIEW + PR number. Commit `"status: <brief-id> in review (#<N>)"` and push.
13. `prs_shipped += 1`. **Iteration jitter**: `sleep $((RANDOM % 26 + 5))`.
14. GO TO STEP 0 (re-read CLAUDE.md, then re-pull main).

============================================================
EMPTY-ITERATION GUARD
============================================================

If `empty_iterations >= 15` (tick369 raised from 3):
- Queue genuinely empty (15 consecutive polls with back-off applied).
- Tell the operator: `"queue empty after 15 polls. Shipped <prs_shipped> PRs this session. Exiting."`
- EXIT cleanly with code 0.

**Workers no longer self-exit on short dry stretches.** Phase A ship-blockers + 165-row queue depth means an empty queue is exceptional. Lean toward continuing to heartbeat with back-off rather than rotating tabs.

Do NOT heartbeat forever on an empty queue. That hides real problems (broken git state, ledger drift) as if they were just an empty queue.

============================================================
STOP CONDITIONS (any one ends the loop)
============================================================

The full list is in CLAUDE.md (STOP CONDITIONS section). Summary:
1. Empty-iteration guard fires (above).
2. Same git operation fails twice for unclear reasons.
3. PRE-LOOP recovery couldn't unblock the workspace.
4. Self-degradation 2+ indicators in one iteration.
5. Operator stop signal: file `/tmp/argo-stop` exists.

When you exit, summarize one line: PRs shipped this session, exit reason, what's left in the queue. Then stop.

============================================================
RULES
============================================================

Full rules in CLAUDE.md under "HARD RULES" — re-read at every iteration step 0. Critical reminders:

- Never edit files outside the current brief's "may touch" list.
- Never modify briefs assigned to claude.
- Never force-push.
- Never delete other agents' branches.
- 3-attempt fail rule: clippy/tests fail unfixably → mark BLOCKED, pick a different row.
- Pre-existing flakes don't block: push with Boundary note (see CLAUDE.md HARD RULE 2).
- Don't loop on broken git state: EXIT and let the operator diagnose.

Do NOT keep looping when the loop is no-op. Exit cleanly so the wrapper or operator can recover.
