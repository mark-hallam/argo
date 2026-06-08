# worker-detail-escalation-policy (FINAL SPRINT) — live escalation read/write + un-mock SPA

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh worker-detail-escalation-policy`.

## Why (evidence)
The worker-detail **Escalation** tab is mock-fed. The tab key/label exist in
`web/components/skycot/WorkerTabs.tsx:26,39` ("escalation"/"Escalation"), but the
content comes from the sample worker-overview path — `web/lib/api/worker-overview.ts`
imports `findSampleWorkerOverview` from `web/lib/sample-worker-overview.ts`, whose
duty/escalation strings (e.g. `sample-worker-overview.ts:124` "Escalate billing
disputes") are persona fixtures, not persisted policy. FEATURE_MATRIX's honesty-gaps
note lists "worker schedules/escalation" as a remaining known gap.

A sibling pattern already exists to mirror exactly:
`crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs` —
`GET /api/v1/workers/<id>/policies` (`list_response`, line 43) +
`PATCH /api/v1/workers/<id>/policies/<action_type>` (`update_response`, line 101),
persisting to `.skycot/worker-policies/<worker_id>.toml` with a cross-tenant-hidden
404 (lines 4-6, 37, 101). This brief adds the **escalation-policy** read/write the
same way and un-mocks the SPA tab under demo-live.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'pub(super) async fn (list_response|update_response)|POLICY_DIR' crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs` — decide: EXTEND this file with an escalations sub-resource, or add a sibling `routes_worker_escalations.rs` mirroring its exact shape (cross-tenant-hidden 404, tenant-scoped path). Do NOT invent a new persistence convention.
- `rg -n 'EscalationPolicy|EscalationRule|struct Escalation' crates/` — if no canonical escalation type exists, declare it ONCE (prefer `argo-types` or the policies module) and re-export; do NOT create parallel copies.
- `rg -n 'dispatch_worker_two_segment|dispatch_worker_subpath' crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs` — register the GET in the same dispatch table `policies`/`retros` use.
- `rg -n 'POST|PATCH' crates/argo-cli/src/handlers/gateway/routes_dispatch_post*.rs` — register the PATCH in the matching POST/PATCH dispatch table.
- `rg -n 'isDemoLive|isAuthBypassed|findSampleWorkerOverview' web/lib/api/worker-overview.ts` — gate the un-mock exactly like the existing modules (live endpoint only when present; sample otherwise).

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs` OR new
  `crates/argo-cli/src/handlers/gateway/routes_worker_escalations.rs`
  (`GET /api/v1/workers/<id>/escalations` + `PATCH /api/v1/workers/<id>/escalations`;
  persist the escalation policy (TOML alongside worker-policies, or a `role_escalation`
  table if SQL is cleaner — additive migration only); audit each write with an
  existing approval/escalation audit kind; tenant-scoped, cross-tenant-hidden 404).
  Keep < 500 LOC.
- the GET + PATCH dispatch tables (register the new routes).
- `crates/argo-types/src` or the policies module (the canonical `EscalationPolicy`
  type, if one must be declared — ONCE).
- (optional) `crates/argo-store/src/migrations/vNN.rs` — ONLY if SQL persistence is
  chosen; additive, next free slot, registered in the registry.
- a tests sibling.
- `web/lib/api/worker-overview.ts` (and/or a small `web/lib/api/escalations.ts`) —
  fetch the live `escalations` endpoint under `isDemoLive()` and fall back to the
  sample only when anonymous / on 404.

## Acceptance (truth-bounded + testable)
- An integration test: `PATCH` an escalation policy for a worker, then `GET` it back
  and assert the persisted policy round-trips (not the fixture); a write audit row is
  appended.
- A **cross-tenant negative test**: a tenant-A caller GETs/PATCHes a tenant-B
  worker id and receives the SAME 404 an unknown worker returns (mirror the
  worker-policies invariant) — no cross-tenant escalation read or write.
- SPA: a vitest proving the escalation module fetches the live endpoint under
  demo-live and only falls back to the sample when anonymous/404.
- `cargo test -p argo-cli` green; `tsc` + the touched-file vitest green.

## On ship
Update the FEATURE_MATRIX worker-escalation row (mock → live read/write) with the
commit SHA + route in the SAME commit, then `python3 scripts/render-feature-matrix.py`.

## Effort
M (Rust + web). ~250–400 LOC incl. tests. Additive (TOML-backed preferred; one
additive migration only if SQL chosen).
