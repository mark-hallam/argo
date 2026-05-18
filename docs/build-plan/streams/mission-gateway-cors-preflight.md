# mission-gateway-cors-preflight — gateway answers CORS preflight (P0 demo unblock)

**Owner**: codex. **Sizing hint**: focused single-crate change + tests.
**Depends on**: none.
**Authored against**: origin/main `91f9d032` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-cors-preflight`.
**Source**: 2026-05-18 browser UX audit — gateway returns 404 on `OPTIONS /api/v1/*` preflight requests. SPA dev server (port 3000) → gateway (port 7400) is cross-origin, so browser sends preflight first; gateway has no handler so browser refuses the real POST. Blocks the demo at scene 2 (workspace-name → Continue → "Failed to fetch").

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — the SPA dev → gateway dev handshake must work locally so anyone running the repo (operator, design partner, prospective dev hire) can drive the product without manual proxy setup.
- **Commercial usability** (#5) — third-party clients (browser extensions, internal tooling, partners) calling the gateway from a different origin need CORS headers to function.

## Scope

P0. Add an OPTIONS preflight handler + `Access-Control-Allow-*` response headers on `/api/v1/*` routes. Lock the contract with two integration tests.

### What lands

1. **OPTIONS preflight handler** for every `/api/v1/*` route. Responds 204 (No Content) with:
   - `Access-Control-Allow-Origin: <Origin>` — echo the request's `Origin` header (or `*` if the allowlist is unset)
   - `Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS`
   - `Access-Control-Allow-Headers: Content-Type, Authorization, X-Argo-Principal, X-Idempotency-Key`
   - `Access-Control-Allow-Credentials: true`
   - `Access-Control-Max-Age: 86400` (24h preflight cache)

2. **Allowlist config** in `argo-config`:
   - `[gateway.cors]` section with `allowed_origins: Vec<String>` (defaults to `["http://localhost:3000", "http://localhost:3001"]` in dev mode, empty in production).
   - Production config explicitly lists known web origins (e.g. `https://app.skycot.com`).
   - If the request `Origin` isn't in the allowlist, return 204 with no CORS headers (browser will block — fail-closed).

3. **CORS headers on regular responses** (not just preflight) — `Access-Control-Allow-Origin: <Origin>` echoed on every 2xx/4xx response for `/api/v1/*` so browsers actually consume the response after the preflight succeeds.

4. **Tests**:
   - Unit test: `OPTIONS /api/v1/auth/signup` with `Origin: http://localhost:3000` returns 204 + correct headers.
   - Unit test: `OPTIONS /api/v1/auth/signup` with `Origin: https://evil.example.com` returns 204 but no `Access-Control-Allow-Origin` (browser will block).
   - Integration test in `crates/argo-cli/tests/cors_preflight_smoke.rs` — boot gateway, send preflight + real POST via curl, assert both succeed.

LOC: ~150.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes.rs` — wire OPTIONS preflight into the dispatcher. ≤30 modified LOC.
- `crates/argo-cli/src/handlers/gateway/cors.rs` (NEW) — preflight + response-header helpers. ≤120 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — module declaration. ≤3 LOC.
- `crates/argo-config/src/sections.rs` (or wherever CORS config lives) — add `[gateway.cors]` section. ≤30 LOC.
- `crates/argo-cli/tests/cors_preflight_smoke.rs` (NEW) — integration test. ≤100 LOC.
- `docs/skycot/operator/cors-config.md` (NEW) — operator runbook documenting the allowlist + how to add new origins. ≤80 LOC.

## Files you must NOT touch

- The CORS spec itself (it's RFC) — implement what browsers actually want.
- Other routes outside `/api/v1/*` (legacy `/sessions/*`, `/audit/*` etc.) — out of scope.
- The SPA's edge routes — those proxy server-side and don't need CORS.

## Acceptance criteria

- [ ] `curl -X OPTIONS http://127.0.0.1:7400/api/v1/auth/signup -H "Origin: http://localhost:3000"` returns 204 with `Access-Control-Allow-Origin: http://localhost:3000` header present.
- [ ] Real browser POST from `http://localhost:3000` to gateway succeeds (preflight + actual request both 2xx).
- [ ] Origin not in allowlist receives 204 without CORS headers (browser-side block).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli --test cors_preflight_smoke` passes.
- [ ] Operator runbook explains how to add an origin to the allowlist.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-gateway-cors-preflight`
- Title: `feat(gateway): CORS preflight handler + allowlist on /api/v1/* (mission-gateway-cors-preflight)`
- Worker verification: crate-scoped clippy + tests + manual curl smoke from a real browser.

## Hidden constraints

- **Echo the Origin, don't wildcard.** Wildcarding `Access-Control-Allow-Origin: *` combined with `Allow-Credentials: true` is rejected by browsers (correctly — would allow CSRF). Echo the actual request Origin if it's allowlisted.
- **Don't break legacy routes.** `/sessions/*` and other non-`/api/v1/*` routes should keep their current behavior (no CORS — same-origin only).
- **Closes the demo-night CORS unblock.** When this lands + the SPA's `NEXT_PUBLIC_GATEWAY_URL` points at the gateway directly, the demo flow works end-to-end.
