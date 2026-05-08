# SKYCOT Swarm Protocol — Live Operational Rules

Single source of truth for swarm behavior. All agent families honor it:
Claude, Codex, and any future coding platform. Updates here propagate
to all agents on the next pull. The worker prompts
(`PROTOCOL_WORKER_PROMPT_*.md`) are platform adapters only; they must
not restate stop thresholds, cargo gates, or ownership rules.

If you are a worker tab, these rules apply on EVERY iteration —
including after auto-compaction summarizes earlier turns. The ledger
is the source of truth, not your memory.

If the operator explicitly designates any agent as the conductor, it
follows the conductor role from `PROTOCOL_LOOP_PROMPT.md`: review/merge
PRs, maintain the ledger, unblock/rewrite briefs, and replenish queues.
A conductor tab must not write feature code; if it previously claimed
worker work, release or finish that worker state before making
conductor mutations.

Companion docs:
- `PROTOCOL.md` — architecture and anti-patterns (the "why").
- `PROTOCOL_LOOP_PROMPT.md` — the conductor tick prompt.
- `CONTRIBUTING_AGENT.md` — short worker companion.
- `KNOWN_FLAKES.md` — current pre-existing test flakes.

## Agent roles

`agent_family` (`claude`, `codex`, or future family) is provenance and
sizing signal, not authority. Every family can run any role if the
operator assigns it and the protocol below allows it.

- **conductor** — global priority, PR review/merge, BLOCKED triage,
  queue replenishment, conflict arbitration, stale package sweeps.
- **worker** — one normal brief at a time through PR creation.
- **manager_worker** — one parent brief, a manager package, bounded
  child decomposition, integration, tests, and one final parent PR.
- **subworker** — one child task inside a manager package; does not
  edit `BUILD_STATUS.md`, spawn descendants, or open the final PR.
- **reviewer** — read-only checker; may inspect diffs and packages,
  but may not edit code, update ledgers, approve, or merge.

Manager-worker mode is opt-in per brief. A brief must say
`Manager-worker allowed: yes` in its `## Parallelism` section and name
a `Max children` value; otherwise the normal worker loop applies. In
manager mode, `BUILD_STATUS.md` tracks only the parent row; child
state lives in `docs/build-plan/work-packages/<brief-id>.md`.
Auto-spawn is allowed only from package `spawn_requests`, when
supported by the platform, within spawn budget, and after child file
locks validate as a subset of the parent may-touch list. `max_depth`
is always `1`, and the conductor remains the only final merge
authority.

## One-time environment setup

Before launching workers, the operator should set:

```bash
export CARGO_TARGET_DIR=/tmp/argo-shared-target
```

Add to `~/.zshrc` or `~/.bashrc`. Without this, every `/tmp/<brief-id>`
worktree gets its own `target/` and rebuilds all 56 crates from
scratch (~28 min cold) every time a worker starts a brief. With the
shared target dir, worktrees reuse compiled crates across briefs;
first brief is cold (~28 min one-time), subsequent briefs touching
1-3 crates compile in ~30s-2min. Cargo's lockfile handles concurrent
worker contention cleanly.

Optional but recommended: `brew install sccache` and
`export RUSTC_WRAPPER=sccache` for cross-machine / cross-tab
compile-cache sharing. Sccache + shared target dir = 5-10× iteration
speedup vs unconfigured.

## HARD RULES (memorize)

1. **Ledger is truth.** Re-pull `origin/main` and re-read
   `docs/build-plan/BUILD_STATUS.md` from disk every iteration. Memory
   is suspect, especially after compaction.

2. **Pre-existing flakes don't block.** If `cargo test` fails on a
   test OUTSIDE your may-touch list AND that test fails on clean
   `origin/main` without your changes, push the PR with a "Boundary
   note" listing the exact failing test ids. Do NOT mark BLOCKED. The
   live flake list lives in `KNOWN_FLAKES.md`.

3. **Don't re-implement merged briefs.** Before claiming, grep
   BUILD_STATUS.md for the brief id; if MERGED on main, pick a
   different row. This is the auto-compaction-amnesia trap.

4. **3-attempt fail rule.** If clippy or tests fail and 3 distinct
   edit-then-rerun cycles can't fix it, mark the brief BLOCKED with a
   one-line reason, commit + push, and pick a different TODO row.

5. **Stay inside may-touch list.** New files in covered directories =
   OK. Unlisted file edits = boundary violation. See "Boundary
   exceptions" below for two standing allowances.

6. **500-LOC per-file invariant.** If a file you must touch is at/over
   500 LOC, do NOT add to it. Split it first OR mark BLOCKED with a
   clear reason ("split needed: file at 500 LOC ceiling"). The
   conductor queues a `claude-<thing>-split` architectural-prep brief
   to unblock. Templates: #851 (worker_actions split), #882
   (cli_subcommands split).

7. **Race-lost handling.** If `git push origin main` fails
   (non-fast-forward):
   - `git pull --ff-only origin main`
   - Re-read BUILD_STATUS.md from disk.
   - Pick a DIFFERENT TODO row — never retry the same row you just lost.
   - Claim the new row + push.

   Or wrap the claim sequence with `bash scripts/claim-with-retry.sh
   <brief-id>` — auto-retries non-fast-forward races up to 3× with
   jitter, exits 1 if the brief is now CLAIMED-by-another, falls
   through to this rule on retries-exhausted.

   **🚨 FOOT-GUN:** NEVER run `git checkout origin/main -- .` in a
   worktree with uncommitted edits — it silently reverts ALL
   tracked-file edits with no warning. Use `git restore <file>`
   per-file or `git stash` BEFORE the checkout. Single-file form
   `git checkout origin/main -- <path>` is safe; the bare-dot form is
   the trap.

8. **Pre-push flight check** (race-collision defense). Before pushing
   your feature branch, in the main checkout (not the worktree):
   - `git fetch origin main`
   - `grep <brief-id> docs/build-plan/BUILD_STATUS.md`
   - If MERGED on main (not CLAIMED-by-you): another tab beat you. Do
     NOT push. Stash any local work and pick a different brief.
   - If still CLAIMED-by-you: proceed.

9. **Never force-push to main.** Force-push to your own feature branch
   (`--force-with-lease`) is OK only after a clean rebase onto current
   main.

10. **Never delete other agents' branches.** Don't `git push origin
    --delete` on any branch you didn't author.

11. **Never modify a CLAIMED brief belonging to another tab.** While a
    brief shows `status=CLAIMED` (or `IN_REVIEW`) on origin/main, that
    tab owns the spec mid-flight. The brief-id prefix (`claude-` /
    `codex-`) is a sizing HINT, not a hard ownership lock — any worker
    may claim any TODO row regardless of prefix.

### Boundary exceptions to HARD RULE 5

Two standing allowances apply automatically:

- **Blanket CLI allowlist.** Every CLI brief may touch `cli.rs`,
  `main.rs`, `handlers/mod.rs`, `tests/`, and `Cargo.toml` under
  strict scope.
- **Exhaustive-match allowance.** If you add an enum variant, you MAY
  touch any existing match across the workspace, but ONLY the new arm.

## Iteration sequence

Run this loop until any STOP CONDITION fires:

1. `cd` to the repo (`~/code/argo` or `argo-codex`).
2. `git checkout main && git pull --ff-only origin main`.
3. Read `docs/build-plan/BUILD_STATUS.md` from disk.
4. Find first row matching: `status=TODO` AND all `depends on`
   predecessors are MERGED. Owner field is a sizing hint, not a lock.
   If queue is empty, see STOP CONDITION 1.
5. **Pre-claim freshness check.** Run `bash
   scripts/check-brief-freshness.sh <brief-id>`. ADVISORY (always
   exits 0); flags drift signals like missing may-touch paths, stale
   stub claims, file-LOC drift, missing post-cli-split sibling files.
   Drift signals → quickly judge: (a) claim and amend may-touch in
   your branch, (b) post a BLOCKED note + pick another, (c) ask the
   conductor to rewrite. Don't dig blind into a stale brief. New
   briefs carry an `**Authored against**: origin/main <sha> on <date>`
   header that the script reads for an extra "N commits behind" signal.
6. **Claim.** Edit BUILD_STATUS.md → set status=CLAIMED, commit
   `"status: claim <brief-id>"`, push to origin/main. Race-lost?
   Apply HARD RULE 7.
7. Read `docs/build-plan/streams/<brief-id>.md` — the spec.
8. Inspect any existing `/tmp/<brief-id>` worktree before deleting.
   Salvage useful uncommitted work; `git stash` if wrong-headed; then:
   ```
   git worktree add /tmp/<brief-id> origin/main
   cd /tmp/<brief-id> && git checkout -b <owner>-<brief-id>
   ```
9. Implement. Stay inside may-touch list. Tick acceptance boxes.
10. **Crate-scoped verification** — see "Verification" below.
11. Apply HARD RULE 8 (pre-push flight check), then `git push -u
    origin <branch>`.
12. `gh pr create` with title and body matching the brief.
13. cd back to main checkout. Edit BUILD_STATUS.md → set row to
    IN_REVIEW with PR number. Commit `"status: <brief-id> in review
    (#<num>)"` and push.
14. Iteration jitter: `sleep $((RANDOM % 26 + 5))`.
15. Loop to step 1.

## Verification

Default to **crate-scoped** checks; workspace commands cost 28+ min
cold and ~9 min warm.

Compute touched-crates from your may-touch list. E.g. for a brief
touching `crates/argo-store/...` and `crates/argo-cli/...`:

```bash
CRATES="-p argo-store -p argo-cli"
cargo clippy $CRATES --all-targets -- -D warnings 2>&1 \
  | grep -E '^(warning|error|note|help|   --> |Finished)' | tail -30
cargo test $CRATES 2>&1 | tail -100
```

`cargo build` is REDUNDANT — clippy already builds. The filtered
clippy form drops ~90% of compile-noise on success while preserving
warnings, errors, source-pointer arrows, and the test-result summary.

**Workspace clippy is ONLY for the final pre-push sanity check.** Run
once after crate-scoped clippy is clean:

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -10
```

If a `tail -30` / `tail -100` cuts off the relevant diagnostic,
re-run unfiltered for the offending crate only:
`cargo clippy -p <crate>` or `cargo test -p <crate>`.

**Why crate-scoped is correct, not lazy.** The brief's may-touch list
defines exactly which crates can change. If your edit doesn't compile
a crate that's NOT in may-touch, that's a HARD RULE 5 boundary
violation. Crate-scoped clippy + workspace final-sanity catches both.
HARD RULE 4 (3-attempt fail) applies.

### Codex pre-push gate

**Codex worker tabs only.** Before `git push -u origin <branch>` (step
11 of the iteration sequence), run `bash scripts/codex-pre-push.sh`
from the worktree. The script runs a blocking workspace clippy and
refuses exit 0 if cargo errors or any `^error` lines surface. It
exists to catch lint regressions introduced *after* the crate-scoped
check ran — most commonly during post-rebase conflict resolution
(documented in #905, where compacted single-line `if`-chains tripped
`collapsible_if`). Claude tabs are not subject to the gate; their
documented failure mode is the 500-LOC ceiling, not clippy
verification gaps.

## Effective worker habits

Patterns from observed high-throughput sessions (4+ PRs/session vs
1-2 PR baseline). Adopt as defaults; deviate consciously.

1. **Validate the brief's premise before claiming (≤30s).** The
   freshness script catches common drift; do a second pass on
   brief-specific assumptions (cited migration files, struct fields,
   paths). If anything doesn't match, post a detailed BLOCKED note and
   pick a different brief. Don't claim then discover staleness
   mid-implementation.

2. **Pattern-match before authoring.** Identify 1-2 closely-analogous
   existing implementations and read them end-to-end before writing
   new code. Examples: new MCP connector →
   `crates/argo-mcp-connector-zendesk/src/*.rs`; new worker CLI handler
   → most recent `worker_*.rs` in `crates/argo-cli/src/handlers/`; new
   schema migration → closest-numbered existing migration + its
   `ensure_schema()` caller. Novelty is the leading source of bugs.

3. **Salvage worktree WIP.** When `git worktree add` shows untracked
   files from a prior aborted attempt, read what's there before
   deleting. Often saves 20-40 min. If wrong-headed, `git stash` for
   conductor review and start clean.

4. **Background long cargo runs.** `run_in_background: true` on any
   command expected to take >30s. Continue context-relevant work
   (read files, draft PR body, plan next step) while it runs. The
   9-min workspace clippy and 28-min cold build are dead time if
   foreground-polled. Use `Monitor` or wait for completion
   notification.

5. **Clean BLOCK + move on (≤2 min budget).** If a brief's premise is
   wrong AND the fix is outside may-touch: write a 3-5 sentence
   BLOCKED note naming the specific gap (which file, which struct,
   which assumption), commit + push, move to the next claimable brief.
   Don't dwell. Trying to push through a wrong brief burns 30+ min.

6. **Sensible deviations are encouraged.** If the brief's CLI surface
   is awkward, its cited path is wrong, or it asks for a type that
   doesn't exist: make the small judgment-call fix and document it in
   the PR body under "Sensible deviations." The brief is a spec, not
   a contract.

7. **Use the swarm's own tooling.** `scripts/check-brief-freshness.sh`,
   `scripts/claim-with-retry.sh`, etc. — they exist because friction
   made them necessary.

8. **Phase-priority discipline.** Drain Phase A first; subsequent
   work compounds on Phase A's tooling.

## Stop conditions

Any one ends the loop cleanly:

1. **Queue empty.** 15 consecutive iterations with no claimable TODO
   row (any owner). Bump empty-poll sleep 60s → 120s after the 3rd
   consecutive empty poll, 240s after the 7th. Workers do not
   self-exit on the first dry stretch.
2. **Same op fails twice.** A git operation that worked previously
   now fails for unclear reasons. Environment is degraded — exit and
   let the operator diagnose.
3. **Auto-compaction warning** (Claude tabs only) — advisory, not
   mandatory. Finish the current brief if mid-flight, then assess: if
   you can still recall the brief's may-touch list and your last 2 PR
   numbers, continue. If recall feels degraded, exit.
4. **Self-degradation: 4+ indicators in one iteration.** See list
   below.
5. **Operator stop.** File `/tmp/argo-stop` exists, OR all rows on the
   ledger are MERGED on main (no remaining TODO/BLOCKED/CLAIMED).

### Self-degradation indicators

- Can't recall the last 2 PR numbers YOU opened in this session.
- Considering claiming a brief that's MERGED on main.
- An operation that passed last iteration now fails for unclear reasons.
- Iteration N taking >2× the wall-clock of iteration N-1 with a
  similar-sized brief.
- Searching for context that should be in your recent memory.

1-3 indicators = continue, but tighten up. 4+ = STOP. A single bad
iteration is not fatal; productive sessions absorb minor friction.

## Graceful exit

Healthy sessions ship 2-4 PRs and exit cleanly. Empirical data across
self-exits converges on the observation that meaningful
context-pressure surfaces after roughly 4 multi-crate cycles in one
session. Treat this as expected operational behavior, not a failure
mode. Once you've shipped 2-4 PRs, look for a natural exit point and
rotate.

This is **permission, not mandate.** There is no hard PR cap: a
productive 6-PR session is fine if you're not seeing self-degradation
indicators.

When exiting cleanly:

1. Final commit to BUILD_STATUS.md (or as its own commit on main):
   `"status: <claude|codex>-worker-self-exit after N PRs (reason: <one-line>)"`.
2. Tell the operator: PRs shipped, briefs MERGED/IN_REVIEW/BLOCKED at
   exit, what triggered the exit.
3. Exit. The wrapper or operator restarts a fresh tab.

## Post-compaction recovery

If you sense compaction just fired:

- These rules still apply (CLAUDE.md re-injects, and CLAUDE.md points
  here).
- Re-pull main and re-read BUILD_STATUS.md to recover ledger state.
- Continue from the current iteration — the brief you're inside is
  recoverable from `git status` / your worktree.
- Do NOT panic-exit. Compaction is benign as long as these rules are
  loaded.
