# Mission load-bearing wire — SSO via SAML / OIDC at gateway edge

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #232. Enterprise procurement requires SSO for human operators reaching the gateway. Today: bearer-token-only. Wire OIDC (Google / Microsoft / Okta as IdP) + SAML (legacy enterprise IdPs) at the gateway's authentication edge.

## Scope

NEW `argo-cli` gateway middleware:

1. **OIDC** via `openidconnect` crate — authorization-code flow with PKCE. Endpoints `/auth/login`, `/auth/callback`, `/auth/logout`. Per-tenant IdP registration via `[sso.oidc.<tenant>]` config section.
2. **SAML** via `samael` or `saml2-rs` crate — SP-initiated SSO with AuthnRequest + ACS response. Endpoint `/auth/saml/{tenant}/acs`. Operator imports IdP metadata XML.
3. **Session cookie** signed with the gateway HMAC secret. TTL configurable (default 8 hours).
4. **Backward compat**: existing bearer-token endpoints continue to work; SSO is an additional auth mechanism for human-facing routes only (`/app/*`, `/inbox`).
5. **JIT user provisioning**: on first successful SSO login, create the operator record automatically (works with SCIM brief; SSO + SCIM are complementary).

This is NOT in scope: IdP-initiated SSO (security worse, no operator demand); MFA enforcement (delegated to IdP); password reset (delegated to IdP); social-login providers (Google as OIDC is fine; Facebook/Twitter explicitly out).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` (NEW, ≤ 480 LOC)
- `crates/argo-cli/src/handlers/gateway/sso_oidc_tests.rs` (NEW, sibling, ≤ 350 LOC) — 8 tests with mock IdP
- `crates/argo-cli/src/handlers/gateway/sso_saml.rs` (NEW, ≤ 480 LOC)
- `crates/argo-cli/src/handlers/gateway/sso_saml_tests.rs` (NEW, sibling, ≤ 350 LOC) — 6 tests
- `crates/argo-cli/src/handlers/gateway/sso_session.rs` (NEW, ≤ 200 LOC) — signed-cookie session helper
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` + `routes_dispatch_post.rs` — narrowly scoped: `/auth/*` route arms
- `argo-config/src/sections.rs` — `[sso]` section
- `docs/skycot/RUNBOOK.md` — "SSO setup" with per-IdP snippets

## Files you must NOT touch

- Existing bearer-auth path (`routes_auth.rs`); the audit chain; session-cookie infra in `argo-tui` (this is HTTP-side)

## Acceptance criteria

- [ ] OIDC login → callback → session cookie → authenticated request flow works against a mock IdP
- [ ] SAML SP-initiated flow works against a mock IdP (e.g. shibboleth-testing-idp fixture)
- [ ] Cookie signed with HMAC; tamper detection returns 401
- [ ] JIT provisioning creates operator on first login
- [ ] `[sso] enabled = false` default — no SSO routes mounted; backward compat
- [ ] Per-tenant IdP configs supported (multi-tenant deployments)
- [ ] 8 OIDC + 6 SAML unit tests + 1 integration test each
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-sso-saml-oidc-gateway`
- PR title: `gateway: SSO via SAML + OIDC (mission-sso-saml-oidc-gateway)`

## Notes

- OIDC code-flow with PKCE is the modern standard. SAML is legacy but enterprise sales need it (most large enterprises still SAML).
- Session cookies are HttpOnly, Secure, SameSite=Lax. The HMAC secret is the existing `ARGO_CHANNEL_HMAC_SECRET` (DTMF tokens use the same secret) — sharing the secret is intentional for single-host deployments; multi-host needs per-host secrets (see zero-trust brief).
- Don't reinvent crypto. Use `openidconnect` + `samael` (or equivalent). If both crates are unmaintained, file a follow-up brief; don't roll our own.
