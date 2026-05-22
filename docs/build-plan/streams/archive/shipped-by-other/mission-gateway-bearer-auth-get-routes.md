# mission-gateway-bearer-auth-get-routes — bearer-token auth on GET routes (Phase A follow-up to #946)

**Owner**: claude. **Depends on**: claude-routes-get-split MERGED (routes.rs needs to come down from 500 LOC ceiling first — #946 added the same gate to POST dispatch and the same architectural pattern is required here).

## Why

#946 (mission-gateway-bearer-auth) added `ARGO_GATEWAY_BEARER_TOKEN` enforcement at `routes_dispatch_post::dispatch_post`. **GET routes were explicitly left out of v1**, per the brief: *"GET routes stay anonymous for v1 (operator dashboards depend on them); read auth is a separate brief."*

The remaining read-side gap is a **data-exfil risk on any non-localhost deployment**: `/sessions`, `/audit/recent`, `/messages/recent`, `/costs/recent`, `/memory/*`, etc. all currently leak workspace state to anyone who can reach the port. This brief closes that gap.

## Scope

When `ARGO_GATEWAY_BEARER_TOKEN` is set:
- All GET routes EXCEPT a curated read-only allowlist (`/`, `/health`, `/health/live`, `/health/ready`, `/version`, `/routes`) require a valid `Authorization: Bearer <token>` header.
- The allowlist exists so that liveness probes, load balancers, and the catalog-discovery flow keep working without auth.
- Failed auth → 401 with the same JSON envelope as #946: `{"error":"unauthorized","reason":"missing or invalid bearer token"}`.
- `ARGO_GATEWAY_AUTH_BYPASS=1` overrides for integration tests (matches #946 escape hatch).
- When `ARGO_GATEWAY_BEARER_TOKEN` is unset → no enforcement (dev default — matches #946).

## Approach (concrete)

1. **Extract the auth helper to a shared module.** Currently `check_bearer_auth` lives as a `fn` (private) in `routes_dispatch_post.rs:254`. Move it to a NEW `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (~50 LOC) with `pub(super) fn check_bearer_auth(req: &str) -> Option<(&'static str, String)>`. Update the call site in `routes_dispatch_post.rs` to import via `super::routes_auth::check_bearer_auth`.

2. **Add the read-allowlist check.** In `routes_auth.rs` add a `pub(super) fn is_public_get(req: &str) -> bool` that returns true for the curated set (`GET / `, `GET /health[/...]`, `GET /version`, `GET /routes`). Anything outside that allowlist requires auth when token is configured.

3. **Wire into GET dispatch.** In `routes_dispatch_get.rs` (lands via `claude-routes-get-split`), at the top of `dispatch_get`, add:
   ```rust
   if !is_public_get(req) {
       if let Some(resp) = check_bearer_auth(req) { return Some(resp); }
   }
   ```

4. **No behaviour change when token is unset.** The check_bearer_auth helper already returns `None` if `ARGO_GATEWAY_BEARER_TOKEN` is not set in env — identical dev-default to #946.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (NEW, ~80 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_auth_tests.rs` (NEW, ≤ 200 LOC) — 6 unit tests:
  - no token configured → public + protected GETs both pass through
  - token configured + correct → both pass
  - token configured + wrong on protected GET → 401
  - token configured + wrong on `/health` → still passes (allowlist)
  - token configured + wrong on `/version` → still passes (allowlist)
  - `ARGO_GATEWAY_AUTH_BYPASS=1` lets protected GETs through with wrong token
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — narrowly: replace inline `fn check_bearer_auth` with `use super::routes_auth::check_bearer_auth;` (move the body, no logic change)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` — narrowly: 4-line allowlist-then-auth conditional at top of `dispatch_get`
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: add `mod routes_auth;` decl
- `docs/skycot/RUNBOOK.md` — narrowly: §11 Security: extend the existing `ARGO_GATEWAY_BEARER_TOKEN` paragraph to note GET-side enforcement + the public-allowlist

## Files you must NOT touch

- The 5+ `routes_post_*.rs` siblings — POST auth is already in #946, no per-handler changes
- `routes_orchestrator.rs` — webinbox approval flow has its own `ARGO_WEBINBOX_BEARER_TOKEN` precedent; this brief is separate
- `argo-capability` or `argo-identity` — Principal threading is the next follow-up brief (`mission-gateway-auth-principal-stamping`), not this one

## Acceptance criteria

- [ ] `ARGO_GATEWAY_BEARER_TOKEN=xyz` + `GET /sessions` without `Authorization: Bearer xyz` → 401
- [ ] Same setup + correct token → GET proceeds normally
- [ ] No token configured → all GETs proceed (dev default)
- [ ] `GET /health` always passes (allowlist), even with wrong token
- [ ] `GET /version` always passes (allowlist), even with wrong token
- [ ] `ARGO_GATEWAY_AUTH_BYPASS=1` lets protected GETs through with wrong token
- [ ] 6 unit tests covering all branches
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] `routes_auth.rs` ≤ 100 LOC, every other modified file < 500 LOC
- [ ] RUNBOOK §11 Security paragraph extended

## Branch + PR

- Branch: `claude-mission-gateway-bearer-auth-get-routes`
- PR title: `gateway: bearer-token auth on GET routes (mission-gateway-bearer-auth-get-routes)`

## Notes for the implementer

- This is **strictly a follow-up to #946** — same pattern, same env vars, same 401 envelope, same bypass. Just GET-side.
- The public allowlist is small + curated. **Do not** add new endpoints to it without a separate brief — `/sessions`, `/audit/*`, `/memory/*`, etc. all stay protected.
- Don't add tonic/axum/middleware framework — hand-rolled HTTP stays hand-rolled (matches #946).
- Mission claim: gateway is no longer anonymously **readable** on non-localhost. Closes the second half of the credibility-bleeder that #946 started.
