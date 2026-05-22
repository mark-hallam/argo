# Mission Phase A SHIP-BLOCKER #2 — bearer-token auth on gateway POST routes (rewrite v2)

**Owner**: claude. **Depends on**: nothing (claude-routes-post-split #932 + claude-gateway-mod-split #935 both MERGED — file structure now stable).

> **REWRITE NOTE (tick 269 by conductor)**: original brief assumed Axum
> `Router::route_layer` middleware. The actual gateway is hand-rolled HTTP
> via `TcpListener` + raw stream parsing — no Axum Router exists. Two prior
> tabs BLOCKED on this mismatch. This rewrite reflects the real architecture
> + extends an existing single-token bearer-auth precedent rather than
> building a parallel one.

## Scope

Gateway POST routes today have **zero auth**. The only protection is the
`127.0.0.1:7400` localhost bind. Any non-localhost deployment is anonymously
writable. This is the #2 Phase A ship-blocker — required before any
non-localhost prod deployment.

**Existing precedent to extend**: `routes_orchestrator.rs:216` already implements
single-token bearer auth for the webinbox approval flow:

```rust
let token = header(req, "Authorization") // line 216
if let Ok(expected) = argo_config::cred::get("ARGO_WEBINBOX_BEARER_TOKEN") {
    // ... compare and reject 401 if mismatch
}
```

This brief generalizes that pattern into **dispatch-level middleware** applied
at `routes_dispatch_post::dispatch_post` (top of the POST handler chain) so
every POST endpoint gets uniform auth.

## Approach (concrete)

1. **Add `argo_config::cred::get("ARGO_GATEWAY_BEARER_TOKEN")` lookup to
   `routes_dispatch_post::dispatch_post` (line 42)** — read once at function
   entry. If env unset → auth bypassed (matches dev-default behavior).
2. **If token configured AND `Authorization` header missing/wrong → 401**.
   Return early before dispatching to any handler.
3. **`ARGO_GATEWAY_AUTH_BYPASS=1` escape hatch** for integration tests.
   Document in `RUNBOOK.md` §security.
4. **Audit row stamping is OUT OF SCOPE for v1** — operator's eventual goal
   is per-principal audit, but principal threading needs more design + would
   require touching all 5 `routes_post_*.rs` siblings. v1 just adds the auth
   gate; principal-plumbing is a follow-up brief.
5. **GET routes stay anonymous** for v1 (operator dashboards depend on them);
   read auth is a separate brief.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` —
  add the auth check at function entry (~15 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_tests.rs` (NEW
  if needed; or extend existing) — 4 unit tests:
  - no token configured → POST passes through
  - token configured + correct → POST passes
  - token configured + wrong → 401
  - `ARGO_GATEWAY_AUTH_BYPASS=1` overrides → POST passes even with wrong token
- `docs/skycot/RUNBOOK.md` — narrowly: §security subsection on
  `ARGO_GATEWAY_BEARER_TOKEN` + bypass env

## Files you must NOT touch

- The 5 `routes_post_*.rs` siblings (sessions/messages/runs/costs/judge) —
  no per-handler changes; auth is at dispatch level
- `routes_orchestrator.rs` — the webinbox precedent stays as-is
- `gateway/mod.rs` — already at 192 LOC post-#935, no changes needed
- argo-capability or argo-identity — Principal threading is a follow-up

## Acceptance criteria

- [ ] Setting `ARGO_GATEWAY_BEARER_TOKEN=xyz` + sending POST without
      `Authorization: Bearer xyz` returns 401
- [ ] Same setup + correct token → POST proceeds normally
- [ ] No token configured → POST proceeds (dev default)
- [ ] `ARGO_GATEWAY_AUTH_BYPASS=1` lets POST through even with wrong token
      (integration-test escape hatch)
- [ ] 4 unit tests covering all four branches
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC (routes_dispatch_post.rs at 240 → ~255)
- [ ] RUNBOOK §security subsection added

## Branch + PR

- Branch: `claude-mission-gateway-bearer-auth`
- PR title: `gateway: bearer-token auth on POST dispatch (mission-gateway-bearer-auth)`

## Notes for the implementer

- The existing webinbox precedent uses `argo_config::cred::get(...)` which
  falls back to env. Use the same pattern for consistency.
- Header parser already exists: `header(req, "Authorization")` from
  `routes_orchestrator.rs`. Move it into a shared helper if more than one
  caller needs it; otherwise inline.
- Don't add tonic, axum, or any new framework dep. Hand-rolled HTTP stays
  hand-rolled.
- The 401 response body should be a small JSON envelope:
  `{"error":"unauthorized","reason":"missing or invalid bearer token"}`.
