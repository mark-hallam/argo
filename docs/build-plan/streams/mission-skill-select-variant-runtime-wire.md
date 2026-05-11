# Mission load-bearing wire — wire skill A/B `select_variant` + `record_outcome` into runtime skill load

**Owner**: claude. **Depends on**: `argo-skill::ab_test::select_variant` SHIPPED (at `crates/argo-skill/src/ab_test_stats.rs:69`); `argo-skill::ab_test::record_outcome` SHIPPED (at `crates/argo-skill/src/ab_test.rs:159`); existing skill-invoke path in `argo-agent-runner` (verify in first 5min — likely in `turn_*` modules touching skill bodies).
**Authored against**: origin/main `e44f6e8a` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-select-variant-runtime-wire`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 #76 — "Wire `select_variant` and `record_outcome` into runtime skill load." Today these helpers are only invoked from CLI handlers (`argo skill ab …`). Mission promise: when a skill has multiple variants registered for A/B, the runtime picks via `select_variant(run_id, invocation_id)` and writes the outcome row when the turn settles. Without this wire, A/B experiments live only in the CLI surface and never actually run against worker traffic.

## Scope

1. **Locate the runtime skill-invocation site**. First 5min task: trace where a worker turn resolves a `skill_name` → `skill_body` for execution. Likely lives near where skills are dispatched as tools. Cite file + line in PR body.

2. **At resolution time**, if the named skill has >1 active variant for the current `tenant_id`, call `select_variant(run_id, invocation_id)` to pick A or B. The chosen `variant_id` MUST be threaded into the audit/turn record so the post-turn callback can reference it.

3. **After the turn settles** (success or failure outcome known), call `record_outcome(variant_id, outcome, ...)` to persist the row. Outcome is `Success` / `Failure` per the existing enum.

4. **Single-variant case**: no select_variant call. Just load + invoke. Don't pretend it's an A/B run.

5. **Error handling**: select_variant errors → log `tracing::warn!`, fall back to the default variant (first one returned by the lookup). record_outcome errors → log only; never fail the turn for this side-channel.

6. **NEW CLI** (small): `argo skill ab snapshot <skill_name> [--tenant <t>]` — print variant counts + significance using existing `Significance` helper. Useful for operator inspection ("is the A/B done?").

This is NOT in scope: changing the A/B math; adding new variants beyond the existing 2-variant assumption; auto-promote on significance (separate brief `mission-skill-ab-quality-promote`); ramping-up logic.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-agent-runner/src/turn_skills_runtime.rs` (NEW, ≤ 200 LOC) — sibling module that owns the resolve-and-pick logic. Keeps `turn.rs` under the 500-LOC ceiling.
- `crates/argo-agent-runner/src/turn.rs` OR `turn_helpers.rs` (verify in first 5min) — narrowly scoped: ONE call site to the new resolver at skill-resolution, ONE post-turn hook into record_outcome. ≤ 30 LOC of additions.
- `crates/argo-agent-runner/src/turn_skills_runtime_tests.rs` (NEW, sibling) — 6 unit tests: single-variant-no-ab; two-variants-deterministic-selection; record-outcome-success; record-outcome-failure; select-error-falls-back-to-first; record-error-logs-but-doesn't-fail-turn.
- `crates/argo-agent-runner/Cargo.toml` — narrowly scoped: add `argo-skill` dep IF not already present (verify).
- `crates/argo-cli/src/handlers/skill_ab.rs` — narrowly scoped: ADD `snapshot` subcommand handler (≤ 80 LOC). Reuses existing `Significance` helper.
- `crates/argo-cli/src/cli_skill_cmds.rs` (verify path) — narrowly scoped: ADD `Snapshot` enum variant.

## Files you must NOT touch

- `crates/argo-skill/src/ab_test*.rs` — A/B math + helpers are stable; reuse only
- `crates/argo-store/migrations/**` — no schema change (record_outcome already writes rows)
- Any non-agent-runner/non-cli crate

## Acceptance criteria

- [ ] Runtime calls `select_variant` exactly once per skill invocation when ≥2 variants exist
- [ ] `record_outcome` called exactly once per turn settle for that invocation
- [ ] Single-variant case skips `select_variant` (no wasted call)
- [ ] select_variant error → warn + fall back to first variant; turn proceeds
- [ ] record_outcome error → warn; turn outcome unchanged
- [ ] `argo skill ab snapshot <name>` prints per-variant counts + significance verdict
- [ ] 6 unit tests for the runtime hook + 3 for the snapshot CLI
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-agent-runner -p argo-cli -p argo-skill` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-select-variant-runtime-wire`
- PR title: `agent-runner: wire skill A/B select_variant + record_outcome into runtime skill load (mission-skill-select-variant-runtime-wire)`

## Notes for the implementer

- The `select_variant` helper is **deterministic** on `(run_id, invocation_id)` — same inputs always return same variant. That makes it safe to call without storing the choice; replays produce identical outcomes. Verify this in your tests.
- For `run_id`: use the session_id or the per-worker run id (whichever is stable per worker turn). Document the choice in PR body.
- The snapshot CLI is a NICE-TO-HAVE for operator debug; keep it under 100 LOC and reuse existing helpers. If it bloats the brief, defer to a follow-up.
- BLOCK and request revision rather than touch ab_test.rs internals.
