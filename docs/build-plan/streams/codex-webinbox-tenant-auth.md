# codex-webinbox-tenant-auth — Require JWT for tenant-scoped WS subscriptions; remove global broadcast

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `ea8f41d0` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-webinbox-tenant-auth`.
**Source**: F-003 in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`.

## Scope

P1 security. The legacy websocket inbox path is intentionally cross-tenant — `WsAuth::Legacy` is documented as receiving every tenant's events:

```
crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs:158-180
   "Legacy bearer-token connection. No tenant filtering."
   tenant_key() returns "*"
```

Any legacy bearer token holder can subscribe and observe every tenant's inbox events.

Scope (operator-acceptable approaches):
1. Require JWT for tenant-scoped subscriptions (preferred). Legacy bearer fallback either: (a) returns 401, OR (b) binds to one configured "default" tenant via a new `[gateway] legacy_ws_default_tenant = "..."` config — never the global `"*"`.
2. Remove the `WsAuth::Legacy` arm that uses `"*"` and the corresponding global broadcast path in the wire side.

Decide (1) vs (2) at implementation time based on whether any production deployment still relies on legacy WS. If no callers depend on it, prefer (2) — cleaner code.

LOC: ~120.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs` — refactor `WsAuth`, `authorize`, `bearer_or_unauth`, and `tenant_key` to require an explicit tenant (JWT claim or configured default). ≤80 modified lines.
- `crates/argo-config/src/sections.rs` — narrowly: optional `[gateway] legacy_ws_default_tenant` block IF approach (1b) is chosen. ≤15 modified lines.
- Test sibling: existing tests in `ws_inbox_events.rs` (or a new `ws_inbox_events_tests.rs` sibling if cleaner) — JWT-tenant-scoped + legacy-401-or-default-tenant + no-global-broadcast. ≤60 LOC.

## Files you must NOT touch

- The wire side broadcast publishing (the code that sends events to the global channel) — IF approach (2) removes the global path, this is in scope; otherwise leave alone. Check first.
- JWT verification logic itself (`jwt::verify_jwt`) — consume the existing primitive.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `WsAuth::Legacy` no longer returns `"*"` for `tenant_key()`.
- [ ] A test proves a legacy bearer connection cannot subscribe without an explicit tenant.
- [ ] A test proves JWT connections only receive their `tnt` claim's events.
- [ ] `cargo test -p argo-cli ws_inbox` passes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-webinbox-tenant-auth`
- PR title: `feat(gateway): require explicit tenant for WS inbox subscriptions; remove global broadcast (codex-webinbox-tenant-auth)`

## Notes for the implementer

- The `"*"` global path was an intentional v1 back-compat seam (per the source comment); removing or replacing it is the explicit security fix per the audit's F-003.
- If you find that production code DOES publish to the global broadcast and downstream listeners rely on it, escalate to operator before removing — that's a wider behavior change. The fix would then be approach (1b) with a default tenant binding.
- Per-tenant broadcast registry already exists (the JWT path uses `claims.tnt` as the key); the fix mostly removes the fallback that bypasses it.
