# Argo Coding Agent Instructions

Argo is a commercial AI-agent platform.

The architecture is:

1. Core Platform
2. Vertical Agent Templates

The core platform is already substantial. Do not rebuild it unnecessarily.

Core platform includes:
- runtime
- tools
- provider routing
- memory
- skills
- audit
- gateway
- eval
- forge
- replay
- MCP
- RBAC
- marketplace

Vertical agents must be implemented through templates, not hardcoded
runtime forks.

Vertical-specific behavior belongs in:
- manifests
- workflow definitions
- prompt packs
- memory schemas
- tool permissions
- escalation policies
- evaluation cases
- UI/gateway metadata

Never place receptionist/support/PR/sales-specific logic into core
crates unless the abstraction is reusable across multiple verticals.

Priorities:
1. reliability
2. observability
3. repeatability
4. security
5. commercial usability
6. extensibility

Maintain:
- single-binary architecture
- Rust 1.75 compatibility
- <500 LOC per file
- additive migrations
- tests for every feature
- JSON output for CLI list/show/status commands

---

## Pre-existing core-platform invariants (still in force)

These were the v0.2 hard rules and remain unchanged:

Five non-negotiable invariants:
  1. Polyglot core (Rust core + JSON-RPC plugin ABI; plugins are subprocesses)
  2. Multi-tenant by construction (tenant_id plumbed through every primitive)
  3. Capability-based security with first-class RBAC
  4. Provider manifests, not adapters (TOML, not Rust)
  5. MCP-first integration (8 in-core tools; everything else MCP)

Hard rules — CI rejects PRs that violate any of these:
  - No file > 500 LOC
  - No fn > 80 LOC
  - No struct impl block > 200 LOC
  - No `unsafe` Rust outside marked FFI shims
  - No `unwrap`/`expect` outside `#[cfg(test)]`
  - No global mutable state. AgentDeps is the only injection point.
  - Every public Rust API has rustdoc with at least one example

When in doubt:
  - Subtract before adding. Idiot Index is solution complexity ÷ problem complexity.
  - Question the requirement. If something doesn't make sense, raise it; do not invent.
  - Ship the 80% solution. Working software beats the perfect solution that arrives late.

The master spec is `docs/plan/argo-build-plan.md` (v0.2) for the
core-platform layer. The vertical-template system is documented in
its own design doc once Phase 1 lands.

PR title format: "<phase> — <section> — <one-line summary>"

---

## Project rename — Argo → Skycot

The project will rename from Argo to **Skycot** (domain: skycot.com).
Continue using `argo` everywhere for now (binary, crates, env vars,
~/.argo/, argo.toml). The rename will land as one or two coordinated
PRs: a cheap repo-URL pass and an optional expensive technical-name
pass. Do not start renaming proactively.

---

## SKYCOT Swarm Worker Loop — Load-Bearing Rules

**This section is the single source of truth for swarm worker behavior.**
Both Claude tabs and Codex tabs honor it. Updates here propagate to all
workers on next pull. The worker prompts (`PROTOCOL_WORKER_PROMPT_*.md`)
delegate to this section — they are slim kickoffs, not duplicated rule
sets.

If you are a worker tab, these rules apply on EVERY iteration —
including after auto-compaction summarizes earlier turns. The ledger
is the source of truth, not your memory.

### HARD RULES (memorize)

1. **Ledger is truth.** Re-pull `origin/main` and re-read
   `docs/build-plan/BUILD_STATUS.md` from disk every iteration. Memory
   is suspect, especially after compaction.

2. **Pre-existing flakes don't block.** If `cargo test` fails on a test
   OUTSIDE your may-touch list AND that test fails on clean `origin/main`
   without your changes, push the PR with a "Boundary note" listing the
   exact failing test ids. Do NOT mark BLOCKED. Known flakes:
   - `gateway_smoke::*` (resolved in #858; ignore if still listed)
   - `setup_steps::vertical_step_skips_empty_registry` (env leak)
   - `worker_actions::gtm_engineer_manifest_declares_expected_action_classification`
     (vertical schema mismatch)

3. **Don't re-implement merged briefs.** Before claiming, grep
   BUILD_STATUS.md for the brief id; if MERGED on main, pick a different
   row. This is the auto-compaction-amnesia trap.

4. **3-attempt fail rule.** If clippy or tests fail and 3 distinct
   edit-then-rerun cycles can't fix it, mark the brief BLOCKED with a
   one-line reason, commit + push, and pick a different TODO row.

5. **Stay inside may-touch list.** New files in covered directories =
   OK. Unlisted file edits = boundary violation. Two amendments apply
   automatically:
   - **BLANKET CLI ALLOWLIST**: every CLI brief may touch `cli.rs`,
     `main.rs`, `handlers/mod.rs`, `tests/`, and `Cargo.toml` under
     strict scope.
   - **EXHAUSTIVE-MATCH amendment**: if you add an enum variant, you
     MAY touch any existing match across the workspace, but ONLY the
     new arm.

6. **500-LOC per-file invariant.** If a file you must touch is at/over
   500 LOC, do NOT add to it. Split it first OR mark BLOCKED with a
   clear reason ("split needed: file at 500 LOC ceiling"). Past pattern:
   the conductor queues a `claude-<thing>-split` architectural-prep brief
   to unblock. See #851 (worker_actions split), #882 (cli_subcommands
   split) for templates.

7. **Race-lost handling.** If `git push origin main` fails (non-fast-forward):
   - `git pull --ff-only origin main`
   - Re-read BUILD_STATUS.md from disk
   - Pick a DIFFERENT TODO row — never retry the same row you just lost
   - Claim the new row + push

8. **PRE-PUSH FLIGHT CHECK** (race-collision defense). Before pushing
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
    tab owns the spec mid-flight. Wait until it lands or releases before
    rewriting. The brief-id prefix (`claude-` / `codex-`) is a sizing
    HINT, not a hard ownership lock — any worker tab may claim any TODO
    row regardless of the prefix (tick369 amendment).

### ITERATION SEQUENCE

Run this loop until any STOP CONDITION fires:

1. `cd ~/code/argo` (or `argo-codex` if that's your tree)
2. `git checkout main && git pull --ff-only origin main`
3. Read `docs/build-plan/BUILD_STATUS.md` from disk
4. Find first row matching: `status=TODO` AND all `depends on`
   predecessors are MERGED. **Owner field is a sizing hint, not a lock
   (tick369 amendment) — any tab may claim any TODO.** Codex-prefixed
   briefs lean smaller/single-crate; claude-prefixed briefs lean
   larger/multi-crate, but the prefix is informational. If none
   qualifies and queue is genuinely empty, see STOP CONDITION 1.
5. Claim: edit BUILD_STATUS.md → set status=CLAIMED, commit
   `"status: claim <brief-id>"`, push to origin/main. Race-lost? See
   HARD RULE 7.
6. Read `docs/build-plan/streams/<brief-id>.md`. This is the spec.
7. `git worktree add /tmp/<brief-id> origin/main && cd /tmp/<brief-id>
   && git checkout -b <owner>-<brief-id>` (clean up any stale worktree
   first: `git worktree remove /tmp/<brief-id> --force 2>/dev/null;
   rm -rf /tmp/<brief-id>`).
8. Implement. Stay inside may-touch list. Tick acceptance boxes.
9. **Filter cargo output to keep your context lean.** Run:
   - `cargo build --workspace --message-format=short 2>&1 | tail -10`
   - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -E '^(warning|error|note|help|   --> |Finished)' | tail -30`
   - `cargo test 2>&1 | tail -100`

   The filtered forms drop ~90% of the "Compiling foo v0.0.1" noise on success while preserving warnings, errors, source-pointer arrows (`-->`), and the test-result summary (which lists failed tests). On a failed clippy/test where the filtered tail-30 / tail-100 cuts off the relevant diagnostic, re-run **unfiltered for the offending crate only** — `cargo clippy -p <crate> -- -D warnings` or `cargo test -p <crate>`. Workspace-wide unfiltered runs cost 10K-50K tokens of context and are rarely necessary.

   3-attempt fail rule applies (HARD RULE 4).
10. PRE-PUSH FLIGHT CHECK (HARD RULE 8). If still CLAIMED-by-you,
    `git push -u origin <branch>`.
11. `gh pr create` with title and body matching the brief.
12. cd back to main checkout. Edit BUILD_STATUS.md → set row to
    IN_REVIEW with PR number. Commit `"status: <brief-id> in review
    (#<num>)"` and push.
13. **Iteration jitter**: `sleep $((RANDOM % 26 + 5))` to spread
    finishing collisions across tabs.
14. Loop to step 1.

### STOP CONDITIONS (any one ends the loop cleanly) — tick369 loosened

1. **Queue empty (loosened).** **15 consecutive iterations** with no
   claimable TODO row (any owner). Bump empty-poll sleep from 60s →
   120s after the 3rd consecutive empty poll, and 240s after the 7th
   — back-off, don't burn cycles. With ~150-row queue depth this
   condition should rarely fire; if it does, the conductor will
   replenish; a fresh tab picks up. **Workers no longer self-exit
   on the first dry stretch — keep heartbeating.**
2. **Same op fails twice.** A git operation that worked previously
   now fails for unclear reasons. Environment is degraded — exit and
   let the operator diagnose.
3. **Auto-compaction warning** (Claude tabs only) — **advisory, not
   mandatory exit (tick369).** If Claude Code prints a compaction
   notice, finish the current brief if mid-flight, then assess: if
   you can still recall the brief's may-touch list and your last 2 PR
   numbers, continue. If recall feels degraded, then exit. Don't
   reflexively quit on the first compaction signal.
4. **Self-degradation 4+ indicators.** **Threshold raised from 2 →
   4 (tick369).** A single off-iteration shouldn't end a productive
   session. See list below.
5. **Operator stop.** File `/tmp/argo-stop` exists, OR ALL rows on the
   ledger are MERGED on main (no remaining TODO/BLOCKED/CLAIMED).

### SELF-DEGRADATION INDICATORS (vigilance signals)

- Can't recall the last 2 PR numbers YOU opened in this session
- Considering claiming a brief that's MERGED on main
- An operation that passed last iteration now fails for unclear reasons
- Iteration N taking >2× the wall-clock of iteration N-1 with a similar-sized brief
- Searching for context that should be in your recent memory

1 indicator = continue, but tighten up.
**4+ indicators in one iteration = STOP (tick369 raised from 2).**
1-3 indicators = continue, but tighten up. A single bad iteration is
not fatal; productive sessions absorb minor friction.

### GRACEFUL EXIT PROTOCOL

When exiting cleanly:
1. Make a final commit to BUILD_STATUS.md with a note (or as its own
   commit on origin/main):
   `"status: <claude|codex>-worker-self-exit after N PRs (reason: <one-line>)"`
2. Tell the operator: PRs shipped, briefs MERGED/IN_REVIEW/BLOCKED at
   exit, what triggered the exit.
3. Exit. The wrapper or operator restarts a fresh tab.

### POST-COMPACTION RECOVERY

If you sense compaction just fired (you can't recall details from earlier turns):
- The rules in this section still apply (CLAUDE.md is auto-re-injected).
- Re-pull main and re-read BUILD_STATUS.md to recover ledger state.
- Continue from current iteration — the brief you're inside is
  recoverable from `git status` / your worktree.
- Do NOT panic-exit. Compaction is benign as long as these rules are loaded.
