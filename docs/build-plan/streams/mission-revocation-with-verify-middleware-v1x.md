# mission-revocation-with-verify-middleware-v1x — revoked-tokens infra + verify-middleware as one unit

**Owner**: claude. **Sizing**: ~700-1000 LOC across argo-store + argo-cli + argo-config.
**Depends on**: `claude-rebase-pr-1835-cookies` MERGED (the cookie+CSRF hardening + JWT `jti` claim must land first so this brief can attach revocation enforcement to existing session JWTs).
**Authored against**: origin/main `58d3e8a1` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-revocation-with-verify-middleware-v1x`.
**Priority**: P1 — closes the revocation half of audit P0-1 without shipping a silent vulnerability window.

## LOC-ceiling waiver (operator-locked 2026-05-20)

The 500-LOC-per-file ceiling (CLAUDE.md HARD RULE 6) is **waived for files touched by this brief** to prevent worker rejection of cohesive security work. Rationale: the revocation enforcement primitive (verify-middleware) must land in the same PR as the revocation infrastructure (table + logout-writes) to avoid the silent-vulnerability window the 2026-05-17 HOLD review flagged. Splitting again for LOC-cosmetic reasons would re-introduce that exact gap.

Rules under this waiver:
- Modified files MAY exceed 500 LOC if a clean split is not available within the same module.
- The 80-LOC-per-function ceiling (`check-fn-loc.sh`) **remains in force** — function-level splits are always available.
- Worker should declare in the PR body which files exceeded 500 LOC and why a split would be artificial.
- Conductor will accept the LOC overage at merge time without re-running the file-LOC gate. The pre-merge gate's other checks (parallel-pub-types, migration-slot, workspace clippy) still apply.

Use judgment: if a clean split exists at a real module boundary, do it. If it'd just move lines to satisfy a counter, leave it.

## Why this brief exists

The 2026-05-17 HOLD review on PR #1835 used the word "regression" deliberately: shipping a `revoked_tokens` table + logout-writes WITHOUT a verify-middleware that consults the table on subsequent requests is a silent security gap. Operators logout → table is populated → cookie is cleared client-side → BUT if an attacker captured the JWT before logout, it remains valid until `exp` because no request-path middleware checks the table.

The 2026-05-19 operator-lock asked the original worker to add the middleware in the same PR. They didn't. The 2026-05-20 operator decision splits the cohesive unit into:

- **Sibling brief `claude-rebase-pr-1835-cookies`**: cookie + CSRF + Origin + `jti` only. Ships first.
- **THIS brief**: `revoked_tokens` + logout endpoint + verify-middleware + 5 tests. Ships as ONE unit — never partially.

This is the only structurally honest path to enterprise revocation.

## Scope

Three things land together in one PR. Never two without the third.

### 1. revoked_tokens persistence (`argo-store`)

- New migration (next free slot): `revoked_tokens` table — `jti TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, revoked_at TIMESTAMP NOT NULL, principal TEXT NULL, reason TEXT NULL`.
- `RevokedTokensStore` module — `revoke(jti, tenant_id, principal, reason)`, `is_revoked(jti) -> bool`, `list_for_tenant(tenant_id) -> Vec<RevokedToken>`.
- Lazy `ensure_schema` for back-compat (matches slots 115/116 pattern).
- 5 unit tests covering revoke/is_revoked round-trip, double-revoke is idempotent, cross-tenant isolation, list_for_tenant returns only same-tenant, schema-bootstrap idempotent.

### 2. Logout endpoint (`argo-cli/handlers/gateway`)

- `POST /api/v1/auth/logout` — extracts `jti` from the request's `__Host-session` cookie via the verify-middleware (which lands in §3), writes the `jti` to `revoked_tokens` via `RevokedTokensStore`, returns `Set-Cookie: __Host-session=; Max-Age=0` cookie-clear instructions. Emits `AuditKind::LogoutCalled` audit row.
- 4 route tests: happy logout, missing cookie returns 401, invalid signature returns 401, double-logout is idempotent (jti already revoked).

### 3. Verify-middleware enforcement (`argo-cli/handlers/gateway/middleware_session_verify.rs` NEW)

- Reads `__Host-session` cookie.
- Decodes + verifies HS256 signature using the same secret the signing path uses (no key rotation in v1; single secret in config — document the v1.x rotation deferral inline).
- Validates `exp` (reject 401 if expired).
- Reads the `jti` claim, queries `revoked_tokens` — if present, reject 401 with `{"error": "session revoked"}`.
- Returns parsed `Claims` to the caller on success; caller passes tenant_id / principal downstream.
- Mounted in `routes.rs::dispatch_route_inner` BEFORE any authenticated route. Magic-link verify (`/auth/verify`), signup (`/auth/signup`), and the WS Ed25519 path are exempted.
- 5 unit tests in `middleware_session_verify::tests`: valid cookie → Claims; missing cookie → 401; bad signature → 401; expired exp → 401; **valid sig + valid exp + jti revoked → 401** (the load-bearing test for the enterprise gate).

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__revoked_tokens.sql` (NEW)
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) + `migrations/mod.rs` + `migrations/registry_compliance.rs` (bump schema version)
- `crates/argo-store/src/revoked_tokens.rs` (NEW) — CRUD + ensure_schema
- `crates/argo-store/src/revoked_tokens_tests.rs` (NEW) — 5 tests
- `crates/argo-store/src/lib.rs` — `pub mod revoked_tokens;` + re-export
- `crates/argo-cli/src/handlers/gateway/middleware_session_verify.rs` (NEW) — the HS256 verify middleware. May exceed 500 LOC if needed (waiver above); ~120-180 LOC target.
- `crates/argo-cli/src/handlers/gateway/middleware_session_verify_tests.rs` (NEW) — 5 tests
- `crates/argo-cli/src/handlers/gateway/routes_auth_logout.rs` (NEW or extend if exists) — `POST /api/v1/auth/logout` handler
- `crates/argo-cli/src/handlers/gateway/routes_auth_logout_tests.rs` (NEW) — 4 route tests
- `crates/argo-cli/src/handlers/gateway/routes.rs` — mount middleware_session_verify; ~10 LOC. May push file over 500 (waiver above).
- `crates/argo-cli/src/handlers/gateway/mod.rs` — register new sub-modules
- `crates/argo-audit/src/event.rs` — add `AuditKind::LogoutCalled` variant + label
- `crates/argo-config/src/sections_security.rs` (or `sections.rs`) — add HS256 secret field if not already present from the cookies brief
- `crates/argo-config/src/lib.rs` — re-export the secret field

## Files you must NOT touch

- Anything from the cookies sibling brief that's already MERGED (cookie minting, CSRF middleware, Origin middleware, jti claim emission). Those are the verify-middleware's PRECONDITIONS; don't rework them.
- WS Ed25519 verify path (`jwt.rs::verify_jwt`) — separate concern.
- OIDC verify (`sso_oidc_jwt::verify_id_token`) — separate concern.
- Anything in argo-replay, argo-genesis, argo-skill — out of scope.

## Acceptance criteria (the load-bearing list)

- [ ] `revoked_tokens` table + CRUD work; `is_revoked` returns `true` after `revoke`, `false` before.
- [ ] `POST /api/v1/auth/logout` writes the cookie JWT's `jti` to `revoked_tokens` and clears the cookie. Returns 401 if cookie is missing or invalid.
- [ ] `middleware_session_verify` rejects requests with valid signature + valid exp + revoked `jti` (the load-bearing case — `session_revoked_jti_rejected` test).
- [ ] `middleware_session_verify` mounted in `routes.rs` before all authenticated routes.
- [ ] Magic-link verify, signup, and WS Ed25519 paths remain exempt from the new middleware.
- [ ] `AuditKind::LogoutCalled` audit row emitted on every logout.
- [ ] All 5 middleware tests + 4 logout tests + 5 revoked_tokens tests pass.
- [ ] `cargo clippy -p argo-store -p argo-cli -p argo-audit -p argo-config --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] `cargo test -p argo-store revoked_tokens` + `cargo test -p argo-cli middleware_session_verify routes_auth_logout` pass.
- [ ] LOC overages on touched files declared in PR body (per waiver above).
- [ ] BUILD_STATUS.md updated with PR number on IN_REVIEW.

## Branch + PR

- Branch: `claude-mission-revocation-with-verify-middleware-v1x`
- PR title: `feat(auth): revoked-tokens infra + HS256 cookie-JWT verify middleware + logout endpoint (mission-revocation-with-verify-middleware-v1x)`

## Notes for the implementer

- This brief's value depends on the THREE pieces landing together. If you can't fit them in one PR for some reason, surface it as a BLOCKED in BUILD_STATUS.md with the structural reason — don't ship the table without the middleware.
- The HS256 secret config field: check `crates/argo-config/src/sections_security.rs` to see if the cookies brief already added it. If yes, reuse. If no, add it — operator-acceptable scope expansion.
- Mount order in `routes.rs::dispatch_route_inner` matters: the verify-middleware goes after the early-exit gates (magic-link verify, signup, WS, healthz, public allowlist) and BEFORE the bearer-token gate. Mirror the existing `middleware_csrf` mount pattern from the cookies brief.
- The `__Host-session` cookie's secret + signing convention was established by the cookies sibling. Your verify-middleware uses the SAME secret to verify the SAME JWTs.
- Test the load-bearing case first: `session_revoked_jti_rejected`. If that test fails, the whole brief's value is missing.

## Out of scope

- Cookie minting / CSRF middleware / Origin middleware — owned by `claude-rebase-pr-1835-cookies`.
- Token rotation, refresh tokens, sliding sessions — separate v1.x briefs.
- OAuth2/OIDC integration — separate.
- A general-purpose "revocation API" beyond logout — separate.
