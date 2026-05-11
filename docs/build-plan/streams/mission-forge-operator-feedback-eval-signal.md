# Mission load-bearing wire — Forge consumes operator feedback as eval signal

**Owner**: claude. **Depends on**: `argo-skill::feedback::{FeedbackVote, should_propose_with_feedback}` SHIPPED (verified at `crates/argo-skill/src/feedback.rs:23`); `argo-store::feedback` storage SHIPPED; `argo-forge::{Forge, ForgeOptions, ForgeRun}` SHIPPED. (`mission-forge-stubforge-cleanup` MAY land first; not a hard dep here.)
**Authored against**: origin/main `d50e69b2` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-forge-operator-feedback-eval-signal`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 #85 — "Build Forge that consumes operator feedback as eval signal." Today Forge runs on synthesised eval cases only; operator thumbs are a parallel signal that influences `should_propose_with_feedback` but never feed into Forge's loss function. Mission promise: when Forge optimizes a skill, it weights operator votes as additional eval-case scores (+1 for Up, –1 for Down) alongside the existing metric-based scoring.

## Scope

1. **`ForgeOptions` extension**: ADD `include_operator_feedback: bool` field (default `true`). When true, Forge ingestion path queries `argo-store::feedback` for votes on this skill's recent action runs.

2. **NEW `argo-forge::feedback_eval` module**: `ingest_operator_feedback(skill_id, window: Duration) -> Vec<FeedbackEvalCase>`:
   - Pulls votes from `argo-store::feedback` for `skill_id` within the time window (default 14 days).
   - Each vote becomes a `FeedbackEvalCase { invocation_id, action_summary, expected: Up→Pass / Down→Fail }`.
   - Aggregated into the eval suite as additional cases tagged `source: "operator_feedback"`.

3. **Loss-function weighting**: when Forge scores a candidate against the eval suite, operator-feedback cases carry weight = 1.5× synthesised cases (operators are higher-signal than synth). Constant lives in `argo-forge::config::FEEDBACK_CASE_WEIGHT` (NEW const, documented).

4. **Provenance in `ForgeRun`**: extend `ForgeRun` with `feedback_cases_count: u32`. Operator can inspect "how many real votes was this run trained against?" via `argo forge show <run_id>`.

5. **Graceful no-feedback case**: if `ingest_operator_feedback` returns empty (no votes in window OR feature flag off), Forge proceeds with synth-only cases unchanged. Logged at `tracing::info!`. NOT an error.

6. **Per-skill opt-out**: `argo skill set-flag <id> --no-forge-feedback` (NEW CLI). Records a per-skill row in NEW `skill_flag` table (TINY migration). Forge consults this flag before ingesting. Useful for skills where operator feedback is noisy.

This is NOT in scope: down-weighting old votes by age (separate brief — `mission-feedback-decay`); cross-skill feedback transfer (separate); penalty-based learning (operator can already block via `Down`).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__skill_flag.sql` (NEW) — small TINY migration:
  ```sql
  CREATE TABLE skill_flag (
      skill_id BLOB NOT NULL,
      flag TEXT NOT NULL,
      value INTEGER NOT NULL,
      PRIMARY KEY (skill_id, flag)
  );
  ```
  Verify next-free slot vs in-flight briefs.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration; bump `EXPECTED_SCHEMA_VERSION` at `crates/argo-store/src/lib.rs:151`.
- `crates/argo-store/src/skill_flag.rs` (NEW, ≤ 200 LOC) — `set_flag` / `get_flag` / `get_flag_or_default`.
- `crates/argo-store/src/skill_flag_tests.rs` (NEW, sibling) — 5 unit tests.
- `crates/argo-forge/src/feedback_eval.rs` (NEW, ≤ 250 LOC) — `ingest_operator_feedback` + `FeedbackEvalCase`.
- `crates/argo-forge/src/feedback_eval_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests: empty-window-returns-empty; mixed-votes-roundtrip; window-respected; skill-flag-disable-skips; aggregate-merges-with-synth; weight-applied-in-loss.
- `crates/argo-forge/src/config.rs` (verify path) — narrowly scoped: ADD `FEEDBACK_CASE_WEIGHT: f32 = 1.5` const.
- `crates/argo-forge/src/run.rs` — narrowly scoped: ADD `feedback_cases_count` field to `ForgeRun` struct + serde plumbing. File currently ≤ 200 LOC; ≤ 25 LOC delta.
- `crates/argo-forge/src/forge.rs` — narrowly scoped: ADD `include_operator_feedback: bool` to `ForgeOptions`; document default = true. ≤ 15 LOC delta.
- `crates/argo-forge/src/lib.rs` — narrowly scoped: re-export `feedback_eval::*`.
- `crates/argo-cli/src/handlers/skill_set_flag.rs` (NEW, ≤ 100 LOC) — CLI for the flag toggle.
- `crates/argo-cli/src/cli_subcommands_skill.rs` (verify path) — narrowly scoped: ADD `SetFlag` variant.

## Files you must NOT touch

- `crates/argo-skill/src/feedback.rs` — `FeedbackVote` + helpers stable; reuse only
- `crates/argo-store/src/feedback.rs` — existing feedback storage stable; reuse only
- Any non-store/non-forge/non-cli crate

## Acceptance criteria

- [ ] Migration adds `skill_flag` table; EXPECTED_SCHEMA_VERSION bumped
- [ ] `ingest_operator_feedback` returns `Vec<FeedbackEvalCase>` from store query
- [ ] Empty-window → empty vec; not an error
- [ ] `include_operator_feedback: false` skips ingestion entirely
- [ ] `skill_flag` row with `no-forge-feedback` skips ingestion for that skill
- [ ] `FEEDBACK_CASE_WEIGHT = 1.5` applied in loss aggregation
- [ ] `ForgeRun.feedback_cases_count` populated correctly
- [ ] `argo skill set-flag <id> --no-forge-feedback` works
- [ ] 5+6 store/forge unit tests + 1 integration smoke (vote → forge → run carries count)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-forge -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-forge-operator-feedback-eval-signal`
- PR title: `forge: ingest operator feedback as weighted eval cases (mission-forge-operator-feedback-eval-signal)`

## Notes for the implementer

- Migration slot: verify against in-flight briefs at claim time. Multiple `mission-skill-*` briefs may stage migrations.
- The 1.5× weight is a starting point. Document as a tunable knob in module rustdoc.
- For the window: 14 days is the v1 default. Make it a parameter on `ingest_operator_feedback` so future cron-based Forge runs can override.
- Don't try to wire auto-cron Forge runs here — that's `mission-forge-cron-lowest-quality`. This brief only opens the ingestion seam.
- If the actual loss-function code lives behind a Python plugin (per `forge.rs` "DSPy/GEPA" note), this brief only needs to wire the Rust-side ingestion + pass the cases via `ForgeOptions`. The Python plugin's reading of cases is a downstream concern.
- The `skill_flag` table is intentionally generic (flag TEXT value INTEGER) so future flags piggyback on the same schema.
