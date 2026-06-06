# fix-worker-detail-route-apply-demo-permission-override

**Owner**: claude.
**Status**: TODO — enqueued 2026-06-06 (conductor) per DEC-040 (operator-authorized; **operator reviews the PR before merge** — RBAC-adjacent).
**Authored against**: origin/main `5e87b3aa5` on `2026-06-06`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-route-apply-demo-permission-override`.

## Why (live-reproduced — GO/NO-GO 2026-06-06)
On a worker's Schedules tab, "Add schedule" + all management is disabled because the
worker **detail** route returns permissions all-false, while the **overview** route
returns them all-true under the same demo principal:
```
GET /api/v1/workers/{id}/overview + demo bearer → can_edit_duties:true …
GET /api/v1/workers/{id}          + demo bearer → can_edit_duties:false …  (all false)
```
The overview route applies `compute_permissions_demo` / `demo_permission_override_allowed`
(`routes_worker_overview.rs`); the **detail** route (`routes_worker_detail.rs`) does
NOT — it only runs `resolve_permissions(role, is_hire_author)`, so a non-author
principal gets nothing. The SPA's `useWorker` hook reads permissions from the
**detail** route → the Schedules tab (and anything else using `useWorker`) is stuck
read-only. Evidence: `docs/skycot/MISSION-E2E-GO-NO-GO-2026-06-06.md` §Blocker-3.

**Precedent:** #2393 (`fix-gateway-demo-mode-worker-permissions-grant-all`, MERGED) added the `compute_permissions_demo` demo-override to the **overview** route only; this brief extends the SAME pattern to the **detail** route, which #2393 did not cover (confirmed gap: `routes_worker_detail.rs` has no demo-override on current main).

This is a **consistency bugfix** (detail should match the override the overview route
already has), NOT a new RBAC policy.

## What
Apply the SAME demo permission-override in `routes_worker_detail.rs` that
`routes_worker_overview.rs` uses. Prefer extracting the existing
`compute_permissions_demo` + `demo_permission_override_allowed` +
`demo_principal_for_request` into a shared helper module both routes call (no
duplicated/parallel logic). When demo mode + a matching demo principal, the detail
route returns the same owner permissions the overview route does.

Grep-before-write: reuse the EXISTING helpers in `routes_worker_overview.rs` — do
NOT re-implement them. `rg 'compute_permissions_demo|demo_permission_override_allowed' crates/`.

## May-touch
- `crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs`
- `crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs` (extract the
  shared helper) + a small shared module if needed.

## Verify
- `GET /api/v1/workers/{id}` with a demo bearer returns permissions matching the
  overview route (all-true under the demo override).
- A test pinning detail-route perms == overview-route perms under demo mode (+ the
  non-demo path unchanged: a non-author principal still gets `resolve_permissions`).
- `cargo test -p argo-cli` (the worker routes) green.
