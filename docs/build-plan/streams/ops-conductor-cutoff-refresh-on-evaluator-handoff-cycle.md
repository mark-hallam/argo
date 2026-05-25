# ops-conductor-cutoff-refresh-on-evaluator-handoff-cycle — conductor cutoff stuck at 14:10Z despite 4+ intervening ticks acknowledging evaluator rounds

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `24cb565b8` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-cutoff-refresh-on-evaluator-handoff-cycle`.

## Why this is P2 (ξ-r69 — NEW process drift)

Evaluator round-69 surfaced a process bug: the conductor "last_run_utc" cutoff in `docs/skycot/agents/heartbeats/conductor.json` was stuck at `2026-05-25T14:10Z` despite multiple intervening conductor ticks (T16-13 / T17-11 / T18-08 + scheduled T19-08) all acknowledging evaluator handoffs r66/r67/r68 in their commit messages. `scripts/check-unread-handoffs.sh` is not idempotent across overlapping evaluator+conductor cycles, OR `PROTOCOL_LOOP_PROMPT.md` Step 4.6 cutoff-refresh is being skipped under load.

Effect: every fresh conductor tick re-flags the same evaluator handoffs as UNREAD, costing time and obscuring genuinely new entries. After T18-08, the cutoff did refresh (so the heartbeat is now fresh as of 2026-05-25T18:13:00Z) — but the load-bearing question is **why** it stayed stuck for 4 rounds and **what guarantees** prevent recurrence.

Round-69 source: `docs/skycot/evals/scheduled/2026-05-25/round-69-process-drift-sweep.md`. Round-67/68 surfaced earlier symptoms; round-69 is the first to name it as ξ.

## Scope

Two-part investigation + fix.

**Part A (investigation, ~20 LOC bash):** Add a `scripts/check-conductor-cutoff-staleness.sh` that reads `docs/skycot/agents/heartbeats/conductor.json`'s `last_run_utc`, compares to `now`, and exits `1` if the cutoff is >2h old. Run this from `scripts/check-unread-handoffs.sh` as a sibling check — if conductor cutoff is stale, emit a `⚠️ conductor cutoff staleness: last refresh Xh ago — review PROTOCOL_LOOP_PROMPT.md Step 4.6 compliance` line.

**Part B (fix, ~10 LOC):** Audit `scripts/check-unread-handoffs.sh` and any helper that touches the cutoff. If the cutoff write is conditional on "all unread entries actioned", that's the bug — a tick that surfaces but cannot fully action entries (e.g., operator-only items) shouldn't leave the cutoff frozen forever. Either:

- (a) Allow informational-only entries to refresh the cutoff (read-only ack), OR
- (b) Add explicit "informational-only — ack without action" path the conductor can invoke.

Implement whichever path the existing script structure makes cleaner. Document the chosen semantics in `PROTOCOL_LOOP_PROMPT.md` Step 4.6.

**Not in scope**: rewriting the heartbeat schema, changing the cutoff source of truth (it stays in `conductor.json`), retroactively fixing past handoff cycles.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/check-unread-handoffs.sh` — Part B.
- `scripts/check-conductor-cutoff-staleness.sh` — new file, Part A.
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — document semantics under Step 4.6.

## Files you must NOT touch

- `docs/skycot/agents/heartbeats/conductor.json` — the script edits it, not this brief directly.
- Any file under `crates/`.

## Acceptance criteria

- [ ] `bash scripts/check-conductor-cutoff-staleness.sh` exits `0` when cutoff is <2h old, `1` when older (smoke test with a temporary back-dated heartbeat fixture).
- [ ] `bash scripts/check-unread-handoffs.sh` no longer leaves the cutoff frozen when all entries are informational/operator-only (verify by running against a SHARED.md that contains only such entries).
- [ ] PROTOCOL_LOOP_PROMPT.md Step 4.6 has explicit guidance on when the cutoff refreshes vs stays frozen.
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Branch + PR

- Branch: `claude-ops-conductor-cutoff-refresh-on-evaluator-handoff-cycle` or `codex-ops-conductor-cutoff-refresh-on-evaluator-handoff-cycle`.
- PR title: `ops: conductor cutoff refreshes on every tick — fixes 4-round stale-cutoff drift (ops-conductor-cutoff-refresh-on-evaluator-handoff-cycle)`.

## Notes for the implementer

- This is bash-only; no Rust. Fast-mode candidate. ~40-60 LOC delta combined.
- Read `scripts/check-unread-handoffs.sh` end-to-end before touching it — the contract is load-bearing for every conductor tick.
- The semantic ambiguity to resolve: "actioned" used to mean "code change / brief flip / merge". Operator-only items (gateway restart, perms, demo recording) cannot be code-actioned by the conductor — they should still ack as read after surfacing.
- Reference round-69 eval report for the live-state proof; this is a NEW finding (1st-round).
