# uf-scheduler-hire-roleduty-bridge (UF Phase-B · B1) — OPERATOR-GATED

**Owner**: claude.
**Depends on**: tenant keystone (#2676); [[DEC-036]] (autonomous scheduled execution).
**Authored against**: origin/main `2133e4511` on `2026-06-06`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-scheduler-hire-roleduty-bridge`.

## 🔒 HOLD — operator-gated (do NOT flip to TODO until released)
Do not build until ALL of: (1) every Phase A brief MERGED, (2) the live signup→hire→work→approve→pay gate passes, (3) Mark explicitly says "go-live Phase B". Per DEC-036/037, Phase B is default-off + reversible. This is the FIRST of B1→B2→B3; a **tenant-isolation adversarial 'try-to-break-it' review** runs before merge.

## Why
Hire writes the `duty` table (`crates/argo-identity/src/store_sql.rs:227` `INSERT INTO duty`), but the scheduler reads `role_duty` (`crates/argo-orchestrator/src/duty_scheduler_store.rs`). No bridge → a hired teammate's cadence is invisible to the scheduler. Make hire populate `role_duty` (the scheduler's source).

## Grep-before-write (verified 2026-06-05)
- `DutyStore::insert` at `duty_scheduler_store.rs:50` — reuse it to write `role_duty` (the same call `routes_duties.rs` already uses). Do NOT author a parallel writer.
- **Reconcile the `tenant_id` TEXT (`duty`) vs BLOB (`role_duty`) representation difference EXPLICITLY** — don't silently mismatch.

## Files you may touch
- `crates/argo-identity/src/store_hire.rs` / `routes_workers_hire.rs`: after creating the worker, write its cadence to `role_duty` via `DutyStore::insert`.

## Acceptance
Hiring a teammate with a schedule creates a `role_duty` row the scheduler can list; tenant-isolated (A's hire ≠ B's, never cwd). Adversarial tenant-isolation review before merge.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (scheduler hire-bridge row) + regen HTML.

## Effort
~1 brief (M).
