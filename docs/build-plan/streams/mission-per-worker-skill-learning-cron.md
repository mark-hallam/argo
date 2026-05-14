# mission-per-worker-skill-learning-cron — Per-worker skill-rating-triggered re-proposal cron

**Owner**: claude. **Depends on**: claude-skill-library-worker-load-wire MERGED (per-worker skills must be loadable before per-worker learning can demonstrate value).
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-per-worker-skill-learning-cron`.

## Scope

Phase A.5 audit found: `argo-store/src/skill_rating.rs` table exists, `argo-skill/src/feedback.rs` collects per-worker skill feedback, and `argo-cli/src/handlers/skill_set_flag.rs` ingests operator feedback into the rating table — **but no cron ever fires to act on those ratings**. The per-worker learning loop is dormant.

This brief adds a cron that:
1. Queries `skill_rating` table per worker.
2. For workers with sufficient feedback volume + positive trend, re-runs proposal logic to surface candidate refinements.
3. Promotes high-feedback skills to the worker's own library tier (per-worker overlay).
4. Logs decisions to the audit chain.

Without this, success scenario #3 (multi-worker handoff continues without re-asking) ships as static: each worker's skill set is what Genesis emitted, not what it learned. Per-worker learning is the "teammate" credibility multiplier.

LOC: ~200.

## Files you may touch

- NEW `crates/argo-skill/src/per_worker_learning_cron.rs` — cron tick logic. Queries skill_rating, runs propose, persists.
- NEW `crates/argo-skill/src/per_worker_learning_cron_tests.rs` — unit tests + fixture-driven smoke.
- `crates/argo-skill/src/lib.rs` — narrowly: `pub mod per_worker_learning_cron;`. ≤5 modified lines.
- `crates/argo-orchestrator/src/cron_registry.rs` (or equivalent cron-registration site) — register the new tick. ≤20 modified lines.
- `crates/argo-store/src/skill_rating.rs` — narrowly: ADD a query function for "ratings since timestamp X" if not present; ≤30 modified lines.

## Files you must NOT touch

- The skill_rating table SCHEMA (additive only; no destructive migrations).
- argo-eval (no eval changes — this is downstream of proposal).
- The argo-skill library lookup logic (separate concern, already wired by Brief A).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Cron tick `per_worker_learning_tick(worker_id, db)` exists, callable in tests.
- [ ] Tick respects threshold (operator-tunable): minimum N ratings + positive trend before re-proposal fires.
- [ ] Re-proposal calls existing `propose_for_review_judged` path (no parallel logic).
- [ ] Promoted skills land in a per-worker overlay tier (does NOT promote into workspace-wide or marketplace-wide tier).
- [ ] Audit-chain emits a per-skill PER_WORKER_LEARNING action.
- [ ] 5+ unit tests covering: threshold, positive trend, mixed-feedback (no fire), cross-tenant isolation, idempotency.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-per-worker-skill-learning-cron`
- PR title: `feat(skill): per-worker skill-rating cron + per-worker overlay tier (mission-per-worker-skill-learning-cron)`

## Notes for the implementer

- Mission-claim alignment: claim 4 (per-worker learning loop) + claim 1 (teammate refinement makes workers feel real over time).
- This is the **second half** of the claim-4 v1 credibility fix. Brief A is the loading; this is the learning.
- Drift-prevention: per-worker overlay is a NEW concept — but it's a tier within existing `LayeredLookup`, not a parallel skill-storage system. Use the existing tier semantics; do not create a new skill table.
