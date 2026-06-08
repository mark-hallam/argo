# teammate-eval-replay-harness (FINAL SPRINT) — store-backed production-outcome replay

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh teammate-eval-replay-harness`.

## Why (evidence)
`crates/argo-eval/src/replay.rs` defines the replay seam but has **no live consumer**:
- `pub trait ProductionOutcomeSource` (line 80) — "reads off audit/run history".
- `pub async fn replay_report(source: &dyn ProductionOutcomeSource, …)` (line 172) —
  merges synthetic + production into one scorecard.
- The ONLY impl is `VecOutcomeSource` (line 210) — in-memory, for tests.

FEATURE_MATRIX explicitly flags "the teammate-eval scorer is still tested code with
zero live consumers." This brief adds the **store-backed** `ProductionOutcomeSource`
(queries live teammate session outcomes) + a read route, turning the scorer into a
real surface.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'pub trait ProductionOutcomeSource|pub async fn replay_report|struct VecOutcomeSource' crates/argo-eval/src/replay.rs` — implement the EXISTING trait; do NOT redefine it.
- `rg -n 'worker_id|scenario|Scenario|outcome' crates/argo-eval/src/replay.rs` — match the exact method signatures the trait requires (query by `worker_id` + `scenario.name`).
- `rg -n 'session.*outcome|run_outcome|fn .*outcomes_for|audit.*by_worker' crates/argo-store/src crates/argo-cli/src` — find the existing store query for session outcomes; reuse it (do NOT add a parallel query layer).
- `rg -n 'GET /api/v1/workers' crates/argo-cli/src/handlers/gateway/mod.rs crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs` — register the new route in the SAME dispatch table the other worker GET routes use.

## Files you may touch
- new `crates/argo-cli/src/handlers/gateway/routes_worker_evals_replay.rs`
  (the store-backed `StoreOutcomeSource impl ProductionOutcomeSource` querying live
  session outcomes by `worker_id` + `scenario.name`, plus a
  `GET /api/v1/workers/<id>/evals/replay` handler that calls `replay_report` and
  returns the merged synthetic+production scorecard JSON). Tenant-scoped (resolve the
  request tenant; a cross-tenant worker id 404s like the existing worker routes).
  Keep < 500 LOC.
- the gateway dispatch table (register the new GET route alongside the other
  `/api/v1/workers/<id>/…` GETs).
- a tests sibling (`routes_worker_evals_replay_tests.rs`).

The store-backed impl MAY live in `argo-cli` (it needs the gateway store), per the
brief instruction; if a more natural home is an existing eval/store crate that
already has the session-outcome query, place it there and re-export — name the
choice at build time. Do NOT add the impl to `argo-eval` if it would pull a store
dependency that crate doesn't already have.

## Acceptance (truth-bounded + testable)
- An integration test: seed N synthetic scenarios + M live session-outcome rows for a
  worker, hit `GET /api/v1/workers/<id>/evals/replay`, assert the returned scorecard
  merges BOTH sources (the production rows shift the score vs synthetic-only) and the
  counts match the seeded data — i.e. `replay_report` actually consumed the
  store-backed source, not an empty vec.
- A cross-tenant negative test: a worker id belonging to tenant B returns the same
  404 an unknown worker returns for a tenant-A caller.
- `cargo test -p argo-cli` green. **Windows-build skip:** if the store query path uses
  a unix-only dep, gate the test `#[cfg(unix)]` and document the skip in the header.

## On ship
Flip the FEATURE_MATRIX teammate-eval-scorer row from "zero live consumers" toward
"Live (store-backed replay + route)" with the commit SHA + route in the SAME commit,
then `python3 scripts/render-feature-matrix.py`.

## Effort
S (Rust). ~150–250 LOC incl. test. Additive; no schema change (reads existing rows).
