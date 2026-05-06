# PROTOCOL_WORKER_PROMPT — paste into a fresh worker tab

The companion to `PROTOCOL_LOOP_PROMPT.md` (which the conductor
runs on a 5-min cron). This doc explains the worker pattern;
the actual paste-ready prompts live in two sibling files so you
don't have to edit anything before pasting:

- **Claude tabs:** [`PROTOCOL_WORKER_PROMPT_CLAUDE.md`](PROTOCOL_WORKER_PROMPT_CLAUDE.md)
- **Codex tabs:** [`PROTOCOL_WORKER_PROMPT_CODEX.md`](PROTOCOL_WORKER_PROMPT_CODEX.md)

The two files are identical except for two lines:
- step 2's `owner == claude` vs `owner == codex` filter
- step 5's `claude-<brief-id>` vs `codex-<brief-id>` branch prefix

The branch prefix gives the conductor at-a-glance provenance on
every PR. Don't deviate.

## Worker prompt (reference copy — Claude variant shown)

For the actual paste-ready text see
`PROTOCOL_WORKER_PROMPT_CLAUDE.md` and
`PROTOCOL_WORKER_PROMPT_CODEX.md`. The summary shape:

1. cd to repo, pull main fresh
2. Re-read BUILD_STATUS.md (do not trust memory)
3. Find first owner-matching TODO row, claim it
4. Read the brief from streams/<brief-id>.md
5. Worktree-add to /tmp/<brief-id>, branch claude- or codex-<brief-id>
6. Implement, respecting may-touch + LOC budget + amendments
7. Run cargo build / clippy / test (skip pre-existing flakes per tick-60)
8. Push branch + open PR with acceptance ticked + Boundary note if any
9. Update BUILD_STATUS.md row to IN_REVIEW with PR number
10. Self-monitor for degradation indicators (no fixed cap)
11. /compact to cap context
12. Re-check stop conditions; loop back to step 1 if healthy

The 4 critical hard rules at the top of each paste-ready file:

- Pre-existing flakes don't block (tick-60 amendment)
- Ledger is source of truth, not memory
- Don't re-implement an already-merged brief (auto-compaction trap)
- Loop endlessly with self-degradation detection (no fixed PR cap)

```
You are a worker tab in the SKYCOT swarm. Implement ONE brief, open a PR, then stop. Do not loop.

Run this exact sequence:

1. cd ~/code/argo && git checkout main && git pull --ff-only origin main

2. Read docs/build-plan/BUILD_STATUS.md. Find the first row where:
   - owner == claude
   - status == TODO
   - all "depends on" predecessors have status == MERGED
   If none match, stop and tell me the queue is empty.

3. Claim the row: edit BUILD_STATUS.md to set that row's status to CLAIMED.
   git commit -m "status: claim <brief-id>" && git push origin main
   If push fails (someone else won the race), pull and pick a different row.

4. Read docs/build-plan/streams/<brief-id>.md. This is the spec.

5. git worktree add /tmp/<brief-id> origin/main
   cd /tmp/<brief-id> && git checkout -b claude-<brief-id>

6. Implement the brief. Strict rules:
   - Stay inside the may-touch list. Adding a new file in a covered directory is fine; touching unlisted files is not.
   - 500-LOC-per-file project invariant. If a file you must touch is at/over 500, split it first OR mark BLOCKED with a clear reason.
   - BLANKET CLI ALLOWLIST: every CLI brief may touch cli.rs / main.rs / handlers/mod.rs / tests / Cargo.toml under strict scope.
   - EXHAUSTIVE-MATCH amendment: if you add an enum variant, you MAY touch any existing match across the workspace, but ONLY the new arm.

7. Run: cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test
   Tick the acceptance boxes in the brief.
   If `cargo test --workspace` fails on a test OUTSIDE your may-touch list AND that test fails on clean origin/main without your changes, that's a pre-existing flake (see BUILD_STATUS.md tick-60 amendment). Do NOT block. Push the PR with a "Boundary note" identifying the failing test by exact id.

8. git push -u origin claude-<brief-id>
   gh pr create --title "<brief PR title>" --body "$(cat <<'EOF'
Brief: docs/build-plan/streams/<brief-id>.md

Acceptance checklist:
[ticked boxes here]

Verification:
- cargo build
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test [scoped command you ran]

Deviations: [none / describe]
EOF
)"

9. cd back to the main checkout (NOT the worktree). Edit BUILD_STATUS.md → set the row to IN_REVIEW with the PR number.
   git commit -m "status: <brief-id> in review (#<num>)" && git push origin main

10. STOP. Do NOT claim a second row. The conductor merges. A fresh tab will pick up the next brief.

Hard rules:
- One PR per tab, then stop.
- Never rebase or force-push main.
- If you hit a wall: edit BUILD_STATUS.md → status BLOCKED with a clear reason, commit + push, then stop. The conductor will unblock you next tick.
- If you can't even claim a row (e.g. queue empty, or everything has unmerged predecessors), stop and tell me.
```

## Why workers loop endlessly (with smart self-degradation detection)

Tick-89 update (final design): there is no fixed PR cap. Workers
loop until they can't safely loop anymore. The mission is
throughput — keep shipping.

**Earlier history:**
- Original design: stop after 1 PR (sledgehammer; wasted operator effort)
- Tick-82 design: stop after 5 PRs (still a sledgehammer; arbitrary cap)
- Tick-89 design: loop endlessly with self-degradation detection (this)

The 5-PR cap was insurance against three failure modes:
stale-state amnesia, /compact drift, auto-compaction wiping
context. But the existing rules already cover these:

1. **Ledger is source of truth, not memory.** Every iteration
   re-pulls main and re-reads BUILD_STATUS.md from disk. Stale
   internal recall is never trusted.

2. **`/compact` between iterations.** Bounds context growth.

3. **Don't reimplement merged briefs.** Defensive rule against
   the auto-compaction-amnesia trap.

4. **Self-degradation detection.** The tab self-monitors for
   indicators of context drift (can't recall recent PR numbers,
   considering already-merged claims, iteration time blowing
   up, mystery test failures, auto-compaction firing
   unprompted). 2+ indicators in one iteration = exit cleanly.

5. **Graceful exit protocol.** When the tab does exit (queue
   empty, degradation detected, hard error), it commits a
   self-exit note to BUILD_STATUS.md so the operator knows
   what happened. A fresh tab spawned with the same prompt
   continues from there.

The tradeoff: workers must self-monitor every iteration. The
hard rules now appear at the top of each prompt as 4 numbered
RULES, with re-read-on-every-iteration explicitly required.

## Spinning up multiple workers

If you want N workers running in parallel:
- Open N tabs.
- Paste this prompt into each.
- They will race on `git push` for the first claim. First push
  wins; losers pull and pick a different row. No coordination
  from you needed.

The lock-free git ledger handles concurrency for free. With ~5
worker tabs you can sustain throughput of one PR every 1-2
minutes, which the conductor (5-min cron) easily merges as
they land.

## Stale claims

If a worker tab dies after CLAIM but before PR, the row sits
CLAIMED with no remote branch. The conductor's tick will
revert stale claims (>30min, no remote branch) back to TODO
on the next sweep. This is the "stale claim sweeper" pattern;
no operator action needed.

## When to stop

The swarm runs until either:
- the operator (you) tells it to stop, or
- every brief in the queue is MERGED AND the e2e test is green
  AND no replenishment ideas remain (full mission delivered)

See `BUILD_PLAN_V11.md` for the standing strategic plan and
`MISSION.md` for the six load-bearing claims that work must
trace back to.
