# Mission production blocker #2 — bearer-token auth on gateway POST routes

**Owner**: claude. **Depends on**: argo-capability crate present (it is).

> SOURCE: deep-research findings tick 198, Section 2, items #18–19.
> Gateway POST endpoints today have ZERO authentication checks
> (verified: `grep -rn "Authorization\|bearer\|auth_required"` on
> routes_post.rs returns nothing). Capability checks happen INSIDE
> the agent loop, not at the HTTP edge. No per-tenant auth, no
> audit trail of who-did-what at HTTP, no RBAC at edge.

## Scope

The gateway runs at `127.0.0.1:7400` by default — localhost-only,
which is the only thing protecting production deployments today.
The instant a customer changes the bind to anything externally
reachable, every POST endpoint is anonymously writable.

This brief adds bearer-token middleware to the gateway. Tokens
map to Principals via the existing `argo-capability` engine.
Every POST endpoint requires a valid token; GET endpoints stay
anonymous for v1 (operator dashboards depend on them; a separate
brief handles read auth).

## Approach

1. Add `argo-capability` as a dep on the gateway path.
2. Write a `bearer_auth_middleware` that reads `Authorization:
   Bearer <token>`, looks up the Principal via a new
   `ArgoConfig::resolve_principal_from_token`, returns 401 on
   miss.
3. Wrap every POST router with the middleware (Axum
   `Router::route_layer`).
4. Plumb the resolved Principal into request handlers via Axum
   extension; existing handlers can ignore it for v1, but the
   audit trail starts capturing it (see acceptance criteria).
5. Tokens are stored under `<workspace>/.argo/tokens.toml` (mode
   0600), each row is `{token_hash, principal, created_at,
   last_used_at}`. New CLI `argo token issue/list/revoke` lives
   in a sibling brief; this brief just reads the file.
6. Backwards compat: env `ARGO_GATEWAY_AUTH_BYPASS=1` (default
   unset) skips auth, used by integration tests. Document loudly.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/auth.rs` (NEW) — middleware
  + token-store reader + Principal resolver
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: register
  the middleware on every POST router branch; add ONE
  `mod auth;` line
- `crates/argo-cli/src/handlers/gateway/routes_post.rs` — narrowly:
  add `Extension<Principal>` extractor to handlers that should
  audit the principal (sessions, forge, vertical, doctor
  dispatch); existing logic unchanged
- `crates/argo-cli/Cargo.toml` — narrowly: add
  `argo-capability = { path = "../argo-capability" }`
- `crates/argo-cli/tests/gateway_auth_smoke.rs` (NEW) — integration
  tests: missing token → 401; bad token → 401; valid token + valid
  call → 200; bypass env → 200 with WARN log

## Files you must NOT touch

- `crates/argo-capability/**` (the engine is locked; consume its
  public API only)
- GET route files (separate brief)
- TLS termination (separate brief — `mission-gateway-tls` queued in
  MISSION_BACKLOG.md item #20)

## Acceptance criteria

- [ ] All gateway POST routes require `Authorization: Bearer <token>`
- [ ] Missing or invalid token returns 401 with a clear body
- [ ] Valid token resolves to a Principal via `ArgoConfig`
- [ ] Principal is plumbed into handlers via Axum Extension
- [ ] Audit row stamped with principal on every successful POST
- [ ] `ARGO_GATEWAY_AUTH_BYPASS=1` skips auth (with WARN log) for
      integration tests; default behavior unchanged
- [ ] `tokens.toml` file mode enforced 0600 on read
- [ ] 6 unit + 3 integration tests
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-gateway-bearer-auth`
- PR title: `gateway: bearer-token auth on POST routes (mission-gateway-bearer-auth)`

## Notes for the implementer

- This is security-sensitive. Use constant-time compare for token
  validation (the `subtle` crate is already a workspace dep via
  the HMAC token path).
- Tokens stored as SHA-256 hashes in `tokens.toml`, never
  plaintext. The CLI brief that issues tokens prints the
  plaintext exactly once.
- The bypass env var is necessary to keep existing integration
  tests passing; document its existence prominently in the auth.rs
  module header so a future security review notices it.
- After this lands, gateway becomes deployment-safe for non-
  localhost bind. TLS is the next layer (separate brief).
