# sprint-ws1-1b — tenant-scope the approval-bundle decide path

**Owner**: claude or codex.
**Depends on**: #2708 (MERGED — migrated list/create; this closes the decide gap it surfaced). Keystone pattern #2676.
**Authored against**: origin/main `b41d0ee82` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-1b-tenant-scope-approval-bundle-decide-path`.

## Why (surfaced by the #2708 adversarial review — verified pre-existing, real)
In `crates/argo-cli/src/handlers/gateway/routes_approval_bundles.rs` (400 LOC), `#2708` migrated
`list_response` (:58) and `create_response` (:93) to `resolve_tenant_for_request`. But
`decide_response` (:120) resolves **no tenant**: it loads the bundle via `get_bundle(store.pool(), bundle_id)`
(:142) by its **global** `bundle_id` UUID, and drives member transitions through cwd-based
`open_workspace_memory()` (:23). Net: a signed-in user can **decide (approve/reject) ANY tenant's
approval bundle** by guessing/holding its UUID, and the member transitions run against the cwd/demo
workspace, not the caller's. That's a cross-tenant write capability on a public multi-tenant launch.
(Pre-existing — `decide` was never in the cwd-resolver set, so #2708 neither caused nor touched it.)

## Task (follow the #2708 / keystone pattern in the SAME file)
1. Resolve the caller's tenant in `decide_response` via `resolve_tenant_for_request(req, store)` (thread
   `req` in if it isn't already a param — `list`/`create` in this file already take it; mirror them).
2. After loading the bundle, **verify the bundle's `tenant_id` == the resolved tenant**. If it does NOT
   match, return **`not_found(bundle_id)`** (same as a missing bundle — do NOT leak existence of another
   tenant's bundle, and do NOT apply any transition).
3. Key the member transitions off the resolved tenant, not cwd `open_workspace_memory()` — scope the
   workspace memory to the resolved tenant (match how a tenant-scoped path opens memory elsewhere; grep
   for an existing tenant-scoped `open_workspace_memory`/memory-for-tenant helper before adding one).

## Verify
- Cross-tenant regression test (mirror WS1-4 #2696): tenant A's session **cannot** decide a bundle owned
  by tenant B → returns `not_found`, no transition applied (assert B's bundle state is unchanged); A can
  decide A's own bundle normally. Must FAIL if decide regresses to the global-by-UUID load.
- `cargo test -p argo-cli -- --test-threads=1` GREEN; clippy clean on touched files (boundary-note the
  pre-existing `await_holding_lock` in `routes_settings_credentials_tests.rs`, untouched).
- Keep the file under the 500-LOC ceiling (400 now — extract a helper if the guard + memory-scoping pushes it near).

## PR
Title: `sprint(ws1-1b) — tenant — scope the approval-bundle decide path (reject cross-tenant decides)`
WS1/security → conductor adversarial-review before merge. Do NOT self-merge.
