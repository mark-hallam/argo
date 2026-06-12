# fix-worker-read-routes-anon-tenant-override

**Owner**: claude or codex — **WS1 TENANT-ISOLATION → conductor adversarial review before merge, do NOT self-merge.**
**Depends on**: none.
**Authored against**: origin/main `218c5fb96` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-worker-read-routes-anon-tenant-override

## Problem (surfaced by the #2858 adversarial review — a PRE-EXISTING platform read-side exposure)

`resolve_tenant_for_request` (`resolve_tenant.rs:138-150`) honors a caller-supplied
`?tenant=<uuid>` / `X-Tenant-Id` override **on the no-session path** (precedence #3).
Several worker-READ routes resolve tenant via this helper but have **no
`enforce_role` gate**, so an UNAUTHENTICATED caller can pass
`?tenant=<victim-uuid>` and read that tenant's data:
- `routes_worker_files` GET list → victim worker's filenames + sizes (found in #2858 review; the upload/DELETE write paths ARE owner-gated — only the GET list is not)
- `routes_worker_detail` → full worker config
- `routes_worker_activity` → activity feed
- `routes_worker_overview` → overview tile
(These leak strictly MORE than the file-list; the file-list is the least of them.)
No write, no file CONTENTS — but a real cross-tenant read of metadata. The
`explicit_tenant_override` anonymous path exists deliberately (#2589) for a
demo/no-auth lane; the gap is that production worker-read routes ride it un-gated.

## Fix (root, ONE place — first-principles)

Prefer the ROOT fix over per-route gating: **require a verified session for the
`?tenant=`/`X-Tenant-Id` override** in production profile — i.e. the anonymous
override resolves to the cwd/demo tenant only, NOT an arbitrary caller-supplied
tenant, when no session is present and the profile is production. Keep the
demo/no-auth lane working in dev (`ARGO_GATEWAY_AUTH_BYPASS`/non-production).
If the root fix is too broad for one brief, FALL BACK to adding
`enforce_role(req, Action::…Read)` (or a require-session check) to the 4 named
worker-read routes + the #2858 file-list, and note the root fix as a sibling.

## May touch
- `crates/argo-cli/src/handlers/gateway/resolve_tenant.rs` (the override gate) — root fix
- OR the 4 worker-read route handlers + routes_worker_files list — per-route fix
- (+ tests)

## Acceptance / verifier
1. Cross-tenant negative test: an UNAUTHENTICATED `GET /api/v1/workers/<B-worker>?tenant=<B>`
   (and the file-list) returns 401/403/404 in production profile — does NOT
   return tenant B's data. Would FAIL on current main.
2. Authenticated owner of tenant B still reads their own worker (no regression).
3. Dev/demo no-auth lane unchanged (`ARGO_GATEWAY_AUTH_BYPASS` path still works).
4. `cargo test -p argo-cli` targeted green; clippy clean.

~80-150 LOC. **🔴 P1 — WS1 cross-tenant read on worker-read routes; close before public launch.**
