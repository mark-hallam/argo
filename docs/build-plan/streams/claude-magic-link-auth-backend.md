# claude-magic-link-auth-backend — Magic-link signup + verify + tenant kickoff (gateway) (Wave 2 P0)

**Owner**: claude. **Depends on**: F-04 `codex-toolcontext-tenant` MERGED (it is) for tenant flow.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-magic-link-auth-backend`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Claim 1 (teammate):** Tenant provisioning is the on-ramp to "your teammate, ready to hire."
- **Pre-condition for self-serve viral readiness.**

## Scope

P0. Two gateway endpoints:

1. `POST /api/v1/auth/signup` — accepts `{ "email": "user@..." }`. Generates 32-byte random token, base64url-encoded, stores in `magic_link_tokens` table with 15-min expiry, sends email via existing `argo-channels::email`. Rate-limit: 5/hour/IP. Returns `{ "status": "sent" }`.

2. `POST /api/v1/auth/verify` — accepts `{ "token": "..." }`. Looks up token, validates not expired + not used, marks used, looks up or creates user, marks email_verified, returns `{ "jwt": "<JWT>", "needs_onboarding": <bool>, "tenant_id": "..." }`. JWT has 7-day expiry; httpOnly cookie set in response.

Plus storage:
- New table `user_account` (id, email, email_verified_at, created_at).
- New table `magic_link_tokens` (token_hash, email, expires_at, used_at).
- New table `user_tenant` (user_id, tenant_id, role) — links user to one or more tenants.

On verify success WITHOUT existing tenant: gateway returns `needs_onboarding: true`. The tenant-provisioning brief (`claude-tenant-provisioning-endpoint`) handles the actual provisioning step.

LOC: ~400.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs` (NEW) — both endpoint handlers. ≤200 LOC.
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_tests.rs` (NEW) — endpoint tests. ≤100 LOC.
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: route mount for `/api/v1/auth/*`. ≤15 modified lines.
- `crates/argo-store/migrations/<next>__user_account.sql` (NEW) — additive migration. Use next monotonic slot per F-02. ≤20 LOC.
- `crates/argo-store/migrations/<next>__magic_link_tokens.sql` (NEW). ≤15 LOC.
- `crates/argo-store/migrations/<next>__user_tenant.sql` (NEW). ≤15 LOC.
- `crates/argo-store/src/user_account.rs` (NEW) — typed accessors. ≤80 LOC.
- `crates/argo-store/src/lib.rs` — narrowly: `pub mod user_account;`. ≤5 modified lines.
- `crates/argo-cli/src/handlers/gateway/rate_limit.rs` — narrowly: add `signup` bucket if not present. ≤30 modified lines.

## Files you must NOT touch

- Existing auth code paths (SSO, OIDC, SAML) — out of scope, parallel system.
- SPA — handled in `claude-spa-signup-flow`.
- Tenant provisioning logic — handled in `claude-tenant-provisioning-endpoint`.

## Acceptance criteria

- [ ] `POST /api/v1/auth/signup` with valid email returns 200 + magic link emailed.
- [ ] Token stored hashed (SHA-256), 15-min TTL, single-use.
- [ ] Rate-limit: 6th signup attempt from same IP within 1hr returns 429.
- [ ] `POST /api/v1/auth/verify` with valid token returns JWT + sets cookie + handles new/existing user.
- [ ] Expired/used/invalid token returns 401 with clear error code.
- [ ] JWT 7-day expiry; httpOnly + secure + SameSite=Lax cookie.
- [ ] New user gets `user_account` row but NO tenant yet — `needs_onboarding: true` flag in response.
- [ ] Migrations land on next monotonic slot (post-F-02 cleanup) — verify slot uniqueness.
- [ ] `cargo test -p argo-cli auth_magic_link` passes.
- [ ] `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-magic-link-auth-backend`
- PR title: `feat(gateway): magic-link signup + verify endpoints (claude-magic-link-auth-backend)`

## Notes for the implementer

- **Token hashing:** store SHA-256 of token, not the token itself. Email contains the raw token; lookup hashes before comparison.
- **Email content:** plain text with clickable HTTPS link. No marketing copy in this email (per Jonny-Ive discipline). Format: "Click to sign in to SKYCOT: <link>. Expires in 15 minutes."
- **Cookie:** name `skycot_session`, scope `*.skycot.com`, httpOnly + secure + SameSite=Lax.
- **Rate-limit:** reuse the existing `rate_limit.rs` bucket pattern.
- **Migration slots:** F-02 (`codex-migration-monotonic-renumber`) will renumber existing collisions; this brief just needs to use the NEXT available slot. Coordinate with F-02 author if collision risk.
- After this brief lands, signup flow is ON; tenant provisioning is the next stop.
