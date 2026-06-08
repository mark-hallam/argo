# self-improvement-cron-driver (FINAL SPRINT) — wire the closed learning library to a driver

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh self-improvement-cron-driver`.

## Why (evidence)
The skill self-improvement library is **complete and tested**, but it has **no driver**
wiring the stages into a live tick — it is library-in-a-box. The closed stages:
- `crates/argo-skill/src/self_improve_loop.rs` — `cluster_to_cohort` (line 102),
  `decide_promotion` (line 220) (the promotion-on-performance gate).
- `crates/argo-skill/src/dreams_distillation.rs` — `distill_and_propose_judged`
  (line 253) — **deterministic fallback path exists; NO live model required**.
- `crates/argo-skill/src/ab_eval.rs` — `run_ab_eval` (line 108) (+ `run_ab_eval_with`
  the deterministic seam at line 114).
- `crates/argo-skill/src/ab_test.rs` — `promote_with_regression_gate` (line 301).
- `cluster_traces` exists (referenced in `self_improve_loop.rs:5` doc-flow).

A per-worker learning cron already exists as a **precedent to mirror**:
`crates/argo-skill/src/per_worker_learning_cron.rs` — `per_worker_learning_tick`
(line 63), `per_worker_learning_tick_with_config` (line 77), `spawn_…_cron`
(line 123), `append_learning_audit` (line 245, hash-chained audit per proposal,
tenant-stamped at line 252). FEATURE_MATRIX marks the learning-loop distillation as
"🟡 placeholder" — this brief makes the cluster→propose→A/B→promote loop *drivable*.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'pub (async )?fn (cluster_traces|cluster_to_cohort|distill_and_propose_judged|run_ab_eval|decide_promotion|promote_with_regression_gate)' crates/argo-skill/src` — call EXISTING fns; do NOT re-implement any stage.
- `rg -n 'per_worker_learning_tick|spawn_per_worker_learning_cron|append_learning_audit' crates/argo-skill/src/per_worker_learning_cron.rs` — mirror this module's tick + audit + tenant-filter shape; do NOT fork a parallel cron type.
- `rg -n 'fn audit_rows_since|last_tick|since.*ts|session.*audit' crates/argo-skill/src crates/argo-store/src` — find the existing "audit rows since cursor" reader; reuse it.
- `rg -n 'pub enum AuditKind' crates/argo-audit/src/event_kind.rs` — if a new per-skill decision audit kind is needed, add it here (additive), not a parallel enum.

## Files you may touch
- new `crates/argo-skill/src/self_improve_cron.rs` (the driver: a
  `self_improve_tick(_with_config)` that reads live session audit rows since the last
  tick → `cluster_traces` → `cluster_to_cohort` → `distill_and_propose_judged`
  (deterministic, no live model) → `run_ab_eval` → `decide_promotion` →
  `promote_with_regression_gate`; emits ONE per-skill decision audit row per
  promotion outcome; tenant-scoped via an `Option<TenantId>` filter like the
  per-worker cron). Keep < 500 LOC; fns < 80 LOC.
- `crates/argo-skill/src/lib.rs` (export the new module).
- ONE entry point — prefer a CLI subcommand handler (e.g. extend an existing
  `argo skill …` handler under `crates/argo-cli/src/handlers/`) OR a gateway route;
  pick whichever already has the store + tenant plumbing with the least new surface.
  Name its consumer in the brief at build time (wire-or-defer).
- `crates/argo-audit/src/event_kind.rs` + `event.rs` (ONLY if a new audit kind is
  needed; additive — add the variant, its string mapping, and the test row).
- a tests sibling (`self_improve_cron_tests.rs`).

## Acceptance (truth-bounded + testable)
- A **unit test** drives the full chain against in-memory fixtures: given seeded
  trace rows that cluster into a cohort with a clear winner, the tick produces a
  `PromotionDecision` and (when promote) calls `promote_with_regression_gate`, and a
  per-skill decision audit row is appended with the tenant stamp.
- An **integration test on the demo seed**: a tick over real seeded session audit
  rows runs end-to-end with the **deterministic** distillation (NO live model call),
  exits 0, and writes ≥1 decision audit row that is greppable by skill id + tenant.
- Tenant isolation: a tick scoped to tenant A never clusters tenant B's traces
  (assert the filter, mirroring the per-worker cron's tenant filter).
- `cargo test -p argo-skill` (and `-p argo-cli` if a CLI handler is added) green.
- Driver does NOT spawn a background loop by default; it is a single tick callable
  on demand (no always-on autonomy — the autofire/graduation flags stay OFF).

## On ship
Flip the FEATURE_MATRIX learning-loop / self-improvement row from "🟡 placeholder"
toward "Live (driver wired)" with the commit SHA + module path in the SAME commit,
then `python3 scripts/render-feature-matrix.py`.

## Effort
M (Rust). ~250–400 LOC incl. tests. Additive; one optional additive migration only
if a tick-cursor needs persistence (prefer reading the existing audit cursor).
