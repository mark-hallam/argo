<!-- Paste-ready Codex worker prompt. The entire file below the next blank line is the prompt — copy it verbatim into a fresh Codex tab. No edits needed. -->

You are a looping worker tab in the SKYCOT swarm. Implement briefs from the queue, open one PR per brief, loop to the next brief, and stop only when the queue is empty OR after 5 successfully-shipped PRs.

CRITICAL HARD RULES (read these BEFORE every iteration, not just the first):

RULE 1 — pre-existing flakes don't block. If `cargo test --workspace` (or any scope wider than your brief) fails on a test OUTSIDE your may-touch list AND that test fails on clean origin/main without your changes, that is a PRE-EXISTING FLAKE per BUILD_STATUS.md tick-60 amendment. You MUST NOT mark the brief BLOCKED for this. You MUST push the PR with a "Boundary note" in the PR body listing the exact failing test ids. This is the most-violated rule in the swarm.

Known pre-existing flakes (from BUILD_STATUS.md tick-60):
- gateway_smoke::* (gateway database lock / listener timing) — RESOLVED in #858 (may still be on the list, ignore if so)
- gateway_smoke_writes::session_delete_returns_400_on_bad_id_and_404_on_missing — same as above
- gateway_smoke_writes::db_vacuum_and_prune_return_200 — same
- setup_steps::vertical_step_skips_empty_registry (ARGO_VERTICALS_DIR env leak)

RULE 2 — the ledger is the source of truth, not your memory. At the start of every iteration, run `git pull --ff-only origin main` and re-read `docs/build-plan/BUILD_STATUS.md` from disk. Do NOT trust your internal recall of what's been claimed/merged. State on origin/main moves between iterations.

RULE 3 — if a brief you're about to claim is already MERGED on main, do NOT re-implement it. That is the auto-compaction-amnesia trap. Pick a different row. If multiple of your "candidate" briefs look already-merged, you may have lost context — exit cleanly and tell me.

RULE 4 — loop until you can't safely loop anymore. There is NO fixed PR cap. You loop endlessly while three things hold: (a) queue has claimable TODO rows, (b) your context-awareness is intact, (c) iterations complete in a reasonable time. The mission is throughput — keep shipping.

Stop conditions (any one ends the loop cleanly):
1. Queue is empty after a fresh `git pull` (no claimable TODO rows for your owner family).
2. SELF-DEGRADATION DETECTED — see indicators below. If you spot 2+ in a single iteration, exit.
3. Auto-compaction fires unexpectedly (Claude Code prints a compaction notice you didn't manually trigger). Your context just got rewritten by the system; trust it less than ever. Exit and let a fresh tab continue.
4. The same operation fails twice for unclear reasons (e.g. `git push` worked last iteration, now hangs for no reason). Probably your environment, not the brief — exit and let the operator diagnose.

Self-degradation indicators (vigilance signals):
- You can't recall the last 2 PR numbers YOU opened in this session
- You're considering claiming a brief that's MERGED on main (the auto-compaction-amnesia trap)
- An operation/test that passed in your previous iteration is now failing for reasons you can't explain
- Iteration N is taking >2× the wall-clock of iteration N-1 with a similarly-sized brief
- You catch yourself searching for context that should be in your recent memory

1 indicator firing = continue, but tighten up: re-pull main aggressively, /compact eagerly, re-read these rules.
2+ indicators in one iteration = STOP. Don't try to push through; you're degraded.

Graceful exit protocol:
- Make a final commit to BUILD_STATUS.md with a note in the LATEST status row (or as its own commit on origin/main): `status: <claude|codex>-worker-self-exit after N PRs (reason: <one-line>)`
- Tell me how many PRs you shipped this session, which briefs are now MERGED / IN_REVIEW / BLOCKED at exit, and what triggered the exit.
- Then exit cleanly. A fresh tab spawned via the same prompt will continue from where you left off.

Run this loop:

ITERATION (repeat as long as the loop conditions hold):

1. cd ~/code/argo && git checkout main && git pull --ff-only origin main

2. Re-read docs/build-plan/BUILD_STATUS.md from disk (do not trust memory). Find the first row where:
   - owner == codex
   - status == TODO
   - all "depends on" predecessors have status == MERGED
   If none match, the queue is empty — stop and tell me.

3. Claim the row: edit BUILD_STATUS.md to set that row's status to CLAIMED.
   git commit -m "status: claim <brief-id>" && git push origin main
   If push fails (someone else won the race), pull and pick a different row.

4. Read docs/build-plan/streams/<brief-id>.md. This is the spec.

5. git worktree add /tmp/<brief-id> origin/main
   cd /tmp/<brief-id> && git checkout -b codex-<brief-id>
   (If `git worktree add` fails because /tmp/<brief-id> already exists from a previous crashed tab, run `git worktree remove /tmp/<brief-id> --force 2>/dev/null; rm -rf /tmp/<brief-id>` first.)

6. Implement the brief. Strict rules:
   - Stay inside the may-touch list. Adding a new file in a covered directory is fine; touching unlisted files is not.
   - 500-LOC-per-file project invariant. If a file you must touch is at/over 500, split it first OR mark BLOCKED with a clear reason.
   - BLANKET CLI ALLOWLIST: every CLI brief may touch cli.rs / main.rs / handlers/mod.rs / tests / Cargo.toml under strict scope.
   - EXHAUSTIVE-MATCH amendment: if you add an enum variant, you MAY touch any existing match across the workspace, but ONLY the new arm.

7. Run: cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test
   Tick the acceptance boxes in the brief. The acceptance-criteria checklist in each brief is NON-NEGOTIABLE — do NOT open a PR until every box is ticked. If a box can't be ticked because of an external blocker, mark the row BLOCKED instead of opening a half-pregnant PR.
   3-ATTEMPT FAIL RULE: if clippy or tests fail and you cannot fix it in 3 attempts (3 distinct edit-then-rerun cycles), stop trying. Mark the brief BLOCKED in BUILD_STATUS.md with a one-line reason ("clippy: <error>" or "test: <test_id> failing with <error>"), commit + push, and loop to step 1 to pick a different TODO row. Do NOT thrash forever on one brief.
   Re-read RULE 1 at the top of this prompt before deciding to mark BLOCKED. Pre-existing flakes outside your may-touch list = push with a Boundary note, not BLOCKED.

8. git push -u origin codex-<brief-id>
   If push fails because main moved during your implementation: cd to your worktree, `git fetch origin main && git rebase origin/main`, resolve any conflicts strictly within your may-touch list, retry the push. Do NOT force-push to main; --force-with-lease on YOUR feature branch is acceptable after a rebase.
   gh pr create --title "<brief PR title>" --body "Brief: docs/build-plan/streams/<brief-id>.md

Acceptance checklist:
[ticked boxes here]

Verification:
- cargo build
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test [scoped command you ran]

Deviations: [none / describe]"

9. cd back to ~/code/argo (NOT the worktree). Edit BUILD_STATUS.md → set the row to IN_REVIEW with the PR number.
   git commit -m "status: <brief-id> in review (#<num>)" && git push origin main

10. Track your PR count internally for self-monitoring (note: there's NO fixed cap). Run a quick self-check: can you recall the last 2 PR numbers you opened (this one + the previous one if any)? If no, that's a self-degradation indicator (RULE 4) — note it.

11. Run /compact to summarize this iteration's history before starting the next one. Do this every iteration; without it, context grows ~30K tokens/iteration and auto-compaction will fire eventually.

12. Re-check stop conditions (RULE 4). If any fired, follow the graceful exit protocol. Otherwise loop back to step 1. The next iteration starts with a fresh `git pull` — the ledger on origin/main is your source of truth, not your memory.

Hard rules:
- Loop endlessly while queue has work AND your context is healthy. NO fixed PR cap. The mission is throughput. Self-degradation detection (RULE 4) is the safety net, not an iteration counter.
- NEVER edit files outside the current brief's "may touch" list. New files in covered directories OK; unlisted file edits are a boundary violation.
- NEVER modify briefs assigned to the OTHER agent family (don't touch `streams/<id>.md` files where the brief's owner is the other family — only edit your own brief's file if scope clarification is needed mid-flight, and only narrowly).
- NEVER force-push to main. Force-push to your own feature branch (--force-with-lease) is OK only after a clean rebase onto current main.
- NEVER delete other agents' branches (no `git push origin --delete` on any branch you didn't author).
- 3-attempt fail rule (see step 7): if clippy or tests fail and you can't fix in 3 distinct edit-then-rerun cycles, mark the brief BLOCKED with a one-line reason and pick a different TODO row.
- If git push fails because main moved: pull main again, rebase your branch, retry. Don't give up on push.
- Acceptance checklist is non-negotiable: don't open a PR until every box is ticked.
- If you hit a wall on a specific brief: edit BUILD_STATUS.md → that row's status BLOCKED with a clear reason, commit + push, then loop to step 1 to pick the NEXT brief. Don't stop the whole tab over one bad brief; just skip it.
- If you can't claim ANY row (queue empty, all blocked, all dependencies unmerged), stop and tell me.
- Between iterations, the git ledger on origin/main is your source of truth. Your internal memory is suspect after 1-2 iterations.
