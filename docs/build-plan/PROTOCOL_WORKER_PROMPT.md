# PROTOCOL_WORKER_PROMPT — paste into a fresh worker tab

The companion to `PROTOCOL_LOOP_PROMPT.md` (which the conductor
runs on a 5-min cron). This prompt makes a Claude or Codex tab
act as a **worker**: claim ONE brief from the ledger, implement
it on a worktree, open a PR, then stop. Workers do NOT loop —
the conductor merges and a fresh tab picks up the next brief.

## Worker prompt (Claude variant)

Paste verbatim into a fresh Claude Code tab. For a Codex tab,
replace `claude` with `codex` in steps 2 and 5.

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

## Codex variant

Same prompt, two replacements:
- Step 2: `owner == codex` (not `owner == claude`)
- Step 5: `git checkout -b codex-<brief-id>` (not `claude-<brief-id>`)

The branch prefix is what gives the conductor at-a-glance
provenance on every PR. Don't deviate.

## Why workers don't loop

A worker tab that claims a second row from the same context has
seen its own first claim and merge — it has a stale view of
what's mergeable, what's blocked, what just landed. The
conductor's job is to maintain that global view; the worker's
job is to do one thing well and stop. Letting workers loop
caused two duplicate-implementation incidents during the v1
build (see PROTOCOL.md "Anti-patterns we hit and how we
recovered" — the H-1 sub-slice double-implementation). One PR
per tab, then stop.

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
