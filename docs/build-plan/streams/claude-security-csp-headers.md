# Mission Phase 2 — Content-Security-Policy + security headers on gateway HTML

**Owner**: claude. **Depends on**: none (independent — extends existing gateway
middleware chain).

## Scope

The gateway serves HTML responses (dashboard, status pages, possibly docs).
Today these responses lack Content-Security-Policy and other browser-side
security headers, which leaves the dashboard vulnerable to XSS, clickjacking,
and content-type confusion. This brief adds a `security_headers` middleware
that injects:

- `Content-Security-Policy` (locked-down for HTML, off for JSON)
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy` (disables geolocation, camera, microphone, payment)
- `Strict-Transport-Security` (only when request was HTTPS)

CSP is the heavy one — it's enforced for HTML responses only (Content-Type
starts with `text/html`); JSON responses skip CSP since browsers won't render
them as documents.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/gateway/security_headers.rs`
   (~220 LOC):
   - `pub async fn security_headers_middleware(req, next) -> Response` —
     axum-style `axum::middleware::from_fn` middleware
   - Calls `next.run(req).await`, then on the resulting response:
     - Adds the always-on headers (X-Content-Type-Options, X-Frame-Options,
       Referrer-Policy, Permissions-Policy)
     - Adds `Strict-Transport-Security: max-age=31536000; includeSubDomains`
       only when the request scheme was HTTPS (read from `x-forwarded-proto`
       or the connection scheme)
     - Adds `Content-Security-Policy` only when response Content-Type starts
       with `text/html`. The CSP value:
       ```
       default-src 'self';
       script-src 'self';
       style-src 'self' 'unsafe-inline';
       img-src 'self' data:;
       connect-src 'self';
       frame-ancestors 'none';
       base-uri 'self';
       form-action 'self'
       ```
       (the `'unsafe-inline'` on `style-src` is a pragmatic concession; if
       the dashboard already uses external CSS only, drop it)
2. **Modify** `crates/argo-cli/src/handlers/gateway/mod.rs` (or whichever file
   wires the gateway router):
   - Layer the middleware via `.layer(axum::middleware::from_fn(security_headers_middleware))`
   - Layer it AFTER auth (auth runs first, headers wrap the response)
3. **Env knob**:
   - `ARGO_CSP_REPORT_ONLY=1` swaps `Content-Security-Policy` for
     `Content-Security-Policy-Report-Only` (for staging rollout)
4. **Audit pass**:
   - Hit the dashboard with the middleware on; confirm no resources are
     CSP-blocked. If they are, tighten the CSP iteratively rather than
     loosening blindly.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/security_headers.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/gateway/security_headers_tests.rs`
  (NEW, ≤ 200 LOC) — 6 unit tests:
  - HTML response gets all headers including CSP
  - JSON response gets all headers EXCEPT CSP
  - HTTPS-derived request adds HSTS
  - HTTP-derived request omits HSTS
  - `ARGO_CSP_REPORT_ONLY=1` switches header name
  - Existing response headers are preserved
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 layer line +
  1 pub mod line
- `crates/argo-cli/tests/gateway_security_headers_smoke.rs` (NEW, ≤ 150 LOC) —
  1 integration test that boots the gateway and asserts headers on a known
  HTML route

## Files you must NOT touch

- Other gateway middleware (auth, rate-limit) — this brief layers alongside,
  not modifies
- Bearer auth implementation (#946) — orthogonal

## Acceptance criteria

- [ ] All HTML responses carry CSP + the always-on headers
- [ ] JSON responses carry the always-on headers (no CSP)
- [ ] HSTS only on HTTPS-derived requests
- [ ] `ARGO_CSP_REPORT_ONLY=1` switches to report-only mode
- [ ] No regressions to existing routes (smoke + clippy clean)
- [ ] 6 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-security-csp-headers`
- PR title: `gateway: CSP + security headers middleware (claude-security-csp-headers)`

## Notes for the implementer

- Reading the request scheme: prefer `x-forwarded-proto` (load-balanced
  deployments) and fall back to the axum connection's scheme. If both are
  absent, omit HSTS (do not assume HTTPS).
- The CSP value above is a starting point; if the dashboard inlines scripts
  the brief should be amended to allow `'unsafe-inline'` on `script-src`
  reluctantly — better to refactor the dashboard to external scripts.
- Keep the middleware purely additive — never replace headers a handler set
  intentionally (a route that explicitly returns its own CSP wins).
