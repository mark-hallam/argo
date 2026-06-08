# worker-detail-schedules-list-get (FINAL SPRINT) — duties list-GET + un-mock schedules panel

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh worker-detail-schedules-list-get`.

## Why (evidence)
The duties **mutations** are live but there is **no list-GET**, so the SPA Schedules
panel renders from persona fixtures. Confirmed:
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_duties.rs:27-33` wires
  only `POST …/duties/{classify,commit,preview}`.
- `crates/argo-cli/src/handlers/gateway/routes_duties.rs` exports
  `classify_response`/`commit_response`/`preview_response`/`patch_response`/
  `delete_response` (lines 31/91/179/242/328) — but **no list/GET** fn.
- `commit_response` persists to the `role_duty` SQL table via `DutyStore`
  (migration `crates/argo-store/src/migrations/v37.rs:49` `CREATE TABLE role_duty`,
  tenant+worker indexed at v37.rs:61-64).
- The SPA client `web/lib/api/duties.ts` only POSTs (classify/preview/commit/update,
  lines 197/216/256/332) and falls back to `mockProposal` / `samplesFor`
  (lines 186/161). There is no `getDuties`/list call, so the panel cannot show
  *persisted* duties — it shows fixtures.

So the gap: add `GET /api/v1/workers/<id>/duties` reading persisted `role_duty` rows
(tenant-scoped), and un-mock the SPA schedules list under demo-live.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'pub(super) async fn|DutyStore|role_duty' crates/argo-cli/src/handlers/gateway/routes_duties.rs` — add a `list_response`; do NOT duplicate the `DutyStore` accessor — reuse it.
- `rg -n 'DutyStore|fn .*list|SELECT .* FROM role_duty|fn duties_for' crates/argo-orchestrator/src crates/argo-cli/src` — find the existing `DutyStore` read helper; if a list query already exists, call it (wire-or-defer); only add a `SELECT … WHERE tenant_id = ? AND worker_id = ?` if none exists.
- `rg -n 'dispatch_worker_two_segment|"duties"|"retros"|"policies"' crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs` — register the GET in the SAME two-segment worker dispatch table as `policies`/`retros`.
- `rg -n 'isDemoLive|mockFallbackAllowed|samplesFor|export async function' web/lib/api/duties.ts` — add the list fetcher with the SAME demo-live / mock-fallback gate the other duties calls use.

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_duties.rs` (add
  `pub(super) async fn list_response(worker_id, req, store)` → returns the persisted
  `role_duty` rows for the worker, tenant-scoped, cross-tenant-hidden 404 like the
  mutations). Keep the file < 500 LOC — if it would exceed, move the list helper to a
  sibling `routes_duties_list.rs`.
- new (only if needed) `crates/argo-cli/src/handlers/gateway/routes_duties_list.rs`.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs` (add the
  `"duties"` arm to the two-segment worker GET dispatch).
- `web/lib/api/duties.ts` (add a `getDuties`/`listDuties` fn that fetches the live
  GET under `isDemoLive()` and falls back to the persona samples only when
  anonymous/404; wire the schedules panel to it).
- a tests sibling + the `duties.test.ts` vitest.

## Acceptance (truth-bounded + testable)
- An integration test: `commit` a duty for a worker, then `GET
  /api/v1/workers/<id>/duties` and assert the committed duty appears in the list
  (reads the real `role_duty` row, not a fixture).
- A **cross-tenant negative test**: a tenant-A caller listing a tenant-B worker's
  duties gets the SAME 404 an unknown worker returns (mirror the mutation invariant).
- SPA vitest: the list fetcher hits the live endpoint under demo-live and only falls
  back to `samplesFor` when anonymous/404.
- `cargo test -p argo-cli` green; `tsc` + `duties.test.ts` green.

## On ship
Update the FEATURE_MATRIX worker-schedules row (mock list → live list-GET) with the
commit SHA + route in the SAME commit, then `python3 scripts/render-feature-matrix.py`.

## Effort
S (Rust + web). ~150–250 LOC incl. tests. Additive; reads the existing `role_duty`
table (no migration).
