# uf-spend-worker-id-attribution (UF Phase-A · A6)

**Owner**: claude.
**Depends on**: tenant keystone (#2676, MERGED).
**Authored against**: origin/main `2264560a2` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-spend-worker-id-attribution`.

## Reconciliation (verified 2026-06-05 — read FIRST)
`session.worker_id` **ALREADY EXISTS** (`crates/argo-store/src/session_worker_id_tests.rs`: `set_session_worker_id`, `get_session().worker_id`, `list_sessions().worker_id`). The stale `claude-session-worker-id-schema` brief is **premise-false / already-done** — do NOT re-add `session.worker_id`. A6 **consumes** the existing field.

## Why
`spend_ledger` has no `worker_id` (`crates/argo-store/src/migrations/v2.rs:34` — verified: 0 `worker_id` refs) → cost can't be split per teammate. Add it additively + thread the existing `session.worker_id` through the spend path. (Per DEC-038, per-teammate cost attribution.)

## Files you may touch (additive ONLY)
- new migration `crates/argo-store/src/migrations/v90.rs` (verified: highest is **v89** → v90 is free): `ALTER TABLE spend_ledger ADD COLUMN worker_id BLOB`. Register it in the migration registry; bump `EXPECTED_SCHEMA_VERSION` 89→90. Additive (no drop/rename) — see `docs/skycot/operator/migrations.md`.
- thread `worker_id` through `SpendCtx` (`crates/argo-agent-runner/src/helpers.rs:127`, built `turn_run_loop.rs:132`) → `record_turn_spend` → `record_spend` (`spend.rs:11`). Source it from the session's existing `worker_id` (duty-fired sessions have it from `DutyRunContext.worker_id`).

## Acceptance
A teammate turn writes a `spend_ledger` row carrying its `worker_id`; a null `worker_id` (system sessions) does NOT break existing rollups; additive migration only (a v89 DB upgrades cleanly to v90).

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (per-teammate cost row, DEC-038) + regen HTML (`python3 scripts/render-feature-matrix.py`).

## Effort
~1–2 briefs (M).
