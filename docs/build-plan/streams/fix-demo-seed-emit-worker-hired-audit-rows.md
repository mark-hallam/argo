# fix-demo-seed-emit-worker-hired-audit-rows

**Owner**: claude or codex.
**Authored against**: origin/main `<latest, post-#2272>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-seed-emit-worker-hired-audit-rows`.
**Depends on**: `mission-demo-seed-real-llm-driver` MERGED (this fix integrates with the driver's audit-emission path).
**Source**: evaluator round-56 P1 finding on PR #2272 (now MERGED).

🟡 **P1 — close the audit-emission gap evaluator round-56 caught.**

Evaluator round-56 on PR #2272 (`mission-demo-seed-crate-scaffold`, now MERGED) flagged a P1 finding: the brief + PR body both claimed `IdentityStore::create_worker` emits baseline `worker.hired` audit rows. **It does not.** `AuditKind::WorkerHired` is emitted only by the gateway hire route at `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs:353-360`.

Result: the seed creates 6 worker rows but **0 audit rows** at hire time. The `/audit` SPA page won't render the "six personas hired" history that Scene 1's narration implies, and the function-vs-dressing rule says rows the camera drills into must be real.

This brief fixes the gap.

## Scope

Two options — pick the cleaner one in the PR body:

**Option A (preferred — additive, minimal)**: in the demo-seed library (`crates/argo-demo-seed/`), after each `IdentityStore::create_worker` call, also emit a `WorkerHired` audit row via the existing `AuditSink::emit` API. Lift the audit-row payload shape from `routes_workers_hire.rs:353-360`.

**Option B (refactor — broader)**: lift the audit emission out of `routes_workers_hire.rs` into `IdentityStore::create_worker` itself, so EVERY caller (gateway + seed + future paths) gets baseline `worker.hired` rows for free. Risk: hire-route callers may not expect the side effect, audit emission needs a tenant-aware sink injection. Defer to a separate brief if chosen.

## Acceptance criteria

- [ ] After `argo demo seed --tenant <demo>`, `curl http://localhost:7400/api/v1/audit?kind=worker.hired&tenant=<demo>` returns 6 rows (one per canon worker: Mariah / Peta / Priya / Jamal / Nori).
- [ ] Each row has a real `worker_id` linking to the seeded worker + real `tenant_id` + valid `payload` shape matching the gateway hire route's emission.
- [ ] Cross-tenant: `curl ...?tenant=<other>` returns 0 rows.
- [ ] `argo audit verify --tenant <demo>` reports `OK` with non-zero rows + zero breaks (Option A only — Option B would touch chain integrity differently).
- [ ] Regression test added under `crates/argo-demo-seed/tests/seed_emits_worker_hired_audit_rows.rs` mirroring the round-56 finding.

## May-touch

- `crates/argo-demo-seed/src/**` (the seed lib)
- `crates/argo-demo-seed/tests/seed_emits_worker_hired_audit_rows.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` (ONLY if Option B chosen)
- This brief file (status updates)

## Out of scope

- Touching the audit chain hash semantics
- Renaming `AuditKind::WorkerHired`
- Refactoring the SPA `/audit` page (separate brief from T09-15 if Scene 6 rendering needs gaps fixed)

## Effort

~50-100 LOC + 1 regression test. ~30-45 min if Option A.
