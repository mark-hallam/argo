# claude-jwt-cookie-security-hardening — pre-launch session-cookie hardening (Wave 4 P1)

**Owner**: claude. **Depends on**: `claude-magic-link-auth-backend` MERGED (IN_REVIEW #1756).
**Authored against**: origin/main `054d8b0a` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-jwt-cookie-security-hardening`.
**Source**: §3 weekend plan Wave 4 launch readiness gap analysis — `gh issue search` for pen-test enterprise procurement no-go.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — drop session-cookie attack surface before public traffic.
- **Security** (CLAUDE.md priority #4) — pre-launch hardening for enterprise procurement.
- **Commercial usability** (#5) — enterprise diligence checks for this exact set of flags.

## Scope

P1. Hardens the session JWT cookie set by the magic-link auth flow. Wave 4 launch-readiness — must land before public signup opens.

Three sub-fixes:

1. **Cookie flag policy.** The cookie set by `POST /api/v1/auth/verify` must use:
   - `HttpOnly` (already set — verify, do not regress).
   - `Secure` (always, even in dev — Next.js dev runs HTTPS via `--experimental-https` or behind proxy).
   - `SameSite=Lax` (NOT `None`, NOT `Strict`).
   - `Path=/` (scope to root — DO NOT scope to `/api`).
   - `Max-Age=<jwt_exp_secs>` matching JWT exp claim.
   - `__Host-` name prefix (`__Host-session`) per RFC 6265bis — enforces Secure + Path=/ + no Domain.

2. **CSRF defense.** For POST / DELETE / PATCH endpoints that mutate state and rely on the session cookie:
   - **Origin check** — middleware validates `Origin` header matches `app.skycot.com` / `api.skycot.com` allow-list. Reject with 403 if mismatch.
   - **Double-submit cookie pattern** — separate `__Host-csrf` cookie set on login containing 32-byte random token; SPA reads it via a same-origin XHR endpoint `/api/v1/auth/csrf` (returns the value) and echoes it in `X-CSRF-Token` header on mutating requests. Middleware compares header vs cookie value. Constant-time compare.

3. **Logout invalidates everywhere.** `POST /api/v1/auth/logout` must:
   - Clear `__Host-session` cookie (`Set-Cookie: __Host-session=; Max-Age=0; ...same flags`).
   - Clear `__Host-csrf` cookie.
   - Insert row in `revoked_tokens` table keyed by JWT `jti` claim with original `exp` timestamp.
   - JWT middleware checks revoked-tokens table (in-memory LRU cache with 60s TTL backed by SQLite read).

LOC: ~250.

## Files you may touch

**Backend:**
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` — extend signup/verify to set new cookie flags + CSRF cookie. ≤60 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs` — JWT middleware adds revoked-token check. ≤40 modified LOC.
- `crates/argo-cli/src/handlers/gateway/middleware_origin_check.rs` (NEW) — Origin-allowlist middleware. ≤80 LOC.
- `crates/argo-cli/src/handlers/gateway/middleware_csrf.rs` (NEW) — double-submit CSRF middleware. ≤90 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_auth_logout.rs` (NEW) — logout endpoint. ≤60 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_auth_csrf.rs` (NEW) — `/api/v1/auth/csrf` token surface. ≤30 LOC.
- `crates/argo-store/migrations/<next>__revoked_tokens.sql` (NEW) — `(jti TEXT PRIMARY KEY, exp INTEGER NOT NULL, revoked_at INTEGER NOT NULL)`. ≤15 LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — mount middleware + new routes. ≤15 modified LOC.

**SPA:**
- `web/lib/auth/csrf-token.ts` (NEW) — fetch & cache CSRF token, expose to fetch helper. ≤50 LOC.
- `web/lib/api/fetch.ts` (existing or NEW) — wire `X-CSRF-Token` header into mutating requests. ≤40 modified LOC.

**Tests:**
- `crates/argo-cli/src/handlers/gateway/routes_auth_tests.rs` — extend with 6 cases (flags present, CSRF allow, CSRF reject mismatch, CSRF reject missing, Origin allow, Origin reject). ≤100 modified LOC.

## Files you must NOT touch

- `crates/argo-identity::jwt` — JWT issuance unchanged. Only the cookie wrapper changes.
- Existing `routes_*.rs` route handlers — middleware is applied at router-layer; per-handler code unchanged.
- Any worker tool surfaces — auth scope only.

## Acceptance criteria

- [ ] `__Host-session` cookie includes `HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=<exp>`.
- [ ] `__Host-csrf` cookie set alongside session cookie on verify success.
- [ ] `GET /api/v1/auth/csrf` returns `{ token: "..." }` (same-origin only).
- [ ] `POST /api/v1/auth/logout` clears both cookies + writes revoked-tokens row.
- [ ] JWT middleware rejects revoked `jti` even within exp window.
- [ ] Mutating requests without `X-CSRF-Token` header → 403.
- [ ] Mutating requests with mismatched `X-CSRF-Token` header → 403.
- [ ] Mutating requests with `Origin` not in allow-list → 403.
- [ ] GET endpoints unaffected by CSRF middleware.
- [ ] SPA `fetch` helper attaches `X-CSRF-Token` automatically.
- [ ] `cargo test -p argo-cli` passes (new + existing auth tests).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck && pnpm test` clean for web changes.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-jwt-cookie-security-hardening`
- Title: `feat(auth): __Host-prefix cookies + CSRF defense + token revocation (claude-jwt-cookie-security-hardening)`
- Worker verification: crate-scoped clippy + tests must pass; PR body declares `clippy --workspace` sanity check passed per SWARM_PROTOCOL.md.

## Hidden constraints

- **`__Host-` prefix is strict** — browsers will reject the cookie if `Path=/` is missing or `Domain=` is set. Test in real browser.
- **CSRF double-submit only works with non-readable header** — must enforce `X-CSRF-Token` not `Cookie: csrf=`, and middleware reads cookie value server-side.
- **Origin check whitelist is env-driven** — `ARGO_TRUSTED_ORIGINS=https://app.skycot.com,https://api.skycot.com` (dev default includes localhost).
- **Revoked-tokens table cleanup** — add nightly cron (or `argo audit gc`) that deletes rows where `exp < now()`. Out of scope for this brief; note as follow-up.
