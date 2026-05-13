# mission-sso-oidc-jwt-signature-verify — verify OIDC ID-token signature via IdP JWKS

**Owner**: claude. **Depends on**: mission-sso-saml-oidc-gateway merged (#1469, after rebase).
**Authored against**: origin/main `b1d78e98` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sso-oidc-jwt-signature-verify`.

> SOURCE: deferred from #1469 PR-body. The OIDC implementation in `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` parses the ID-token payload via base64 decode + JSON parse, **without verifying the JWT signature**. Worker's v1 reasoning is defensible (token arrives back-channel from `token_endpoint` over TLS with `client_secret`), but enterprise auditors will flag this. The IdP exposes a JWKS endpoint; fetching it at startup + verifying the JWT signature on every callback closes the gap.

## Scope

Wire JWT signature verification into the OIDC `/auth/callback` path:

1. **Add `jsonwebtoken = "9"`** (or equivalent — `josekit` is also acceptable; pick the smaller dep tree). Verify dep is not already pulled in workspace-wide before adding.
2. **Fetch the IdP JWKS at startup** (once per tenant) from the IdP's `jwks_uri` (the OIDC discovery doc's standard field). Cache in-memory in `SsoOidcTenant` (the existing per-tenant config struct, in `crates/argo-config/src/sections_sso.rs` per the #1469 file inventory). Refresh cadence: 1h TTL with on-demand re-fetch on signature-validation failure (signing-key rotation tolerance).
3. **Replace `parse_id_token_claims_unverified` (or whatever the unverified parser is named in #1469) with `verify_and_parse_id_token`** that:
   - Splits the JWT into header / payload / signature.
   - Extracts `kid` from the header.
   - Looks up the public key in the cached JWKS.
   - Verifies the signature using `RS256` (the OIDC default; SHOULD also accept `ES256` if the IdP advertises it).
   - Verifies the `iss` claim matches the configured `issuer` for the tenant.
   - Verifies the `aud` claim contains the configured `client_id`.
   - Verifies `exp` is in the future and `iat` is in the past (with a small clock-skew tolerance, ±60s).
   - Verifies the `nonce` claim matches the one minted during `/auth/login` (sourced from the PKCE cookie's nonce field — already plumbed in #1469).
4. **Audit row on every callback**: emit a structured audit event recording: outcome (Verified / SignatureRejected / IssuerRejected / AudRejected / Expired / NonceRejected), `sub`, `kid`, and a short reason string. Use the existing `argo-audit::AuditSink` (already wired in gateway boot).
5. **Test plan**: unit tests for the verification function with golden JWTs (good, bad-sig, expired, wrong-aud, wrong-iss, nonce-mismatch, missing-kid). Integration test that wiremocks the JWKS endpoint + drives one happy-path callback.

**Not in scope**: JWS encryption (JWE — IdPs rarely encrypt OIDC ID tokens; out-of-scope for v1). Custom signing algorithms beyond RS256/ES256. JWKS cache persistence across process restart (in-memory only is fine).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` — narrowly scoped: extend the existing parser to verify-then-parse. ≤120 modified lines.
- `crates/argo-cli/src/handlers/gateway/sso_oidc_tests.rs` — narrowly scoped: ADD 7-8 verification tests. ≤200 added lines.
- `crates/argo-config/src/sections_sso.rs` — narrowly scoped: add `jwks_uri: String` and `jwks_cache: Option<...>` fields to `SsoOidcTenant`. Default `jwks_cache_ttl_secs = 3600`. ≤30 modified lines.
- `crates/argo-cli/Cargo.toml` — narrowly scoped: add ONE `jsonwebtoken = "9"` dep (or chosen equivalent). Verify workspace-wide before adding.
- `<NEW: crates/argo-cli/tests/sso_oidc_jwks_smoke.rs>` (optional, ≤150 LOC) — wiremock-driven happy-path integration test.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/sso_saml.rs` — SAML is a separate brief (`mission-sso-saml-xml-dsig-verify`).
- `crates/argo-cli/src/handlers/gateway/sso_dispatch.rs` — dispatch contract is locked from #1469.
- `crates/argo-cli/src/handlers/gateway/sso_session.rs` — session-cookie HMAC is locked from #1469.
- Any provider crate — JWT verification lives in the SSO handler, not in argo-provider.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `verify_and_parse_id_token(jwt, jwks, tenant_cfg, now, nonce_expected)` returns `Ok(IdTokenClaims { sub, email, ... })` on full success.
- [ ] Wrong signature → `Err(SignatureRejected)`; wrong `iss` → `Err(IssuerRejected)`; wrong `aud` → `Err(AudRejected)`; expired → `Err(Expired)`; nonce mismatch → `Err(NonceRejected)`. All 5 typed errors covered.
- [ ] JWKS cache: fetched at startup; 1h TTL; on signature-validation failure, force-refresh once and retry (signing-key rotation tolerance).
- [ ] Audit row emitted on every callback outcome (success + each failure mode).
- [ ] 7+ unit tests (good, bad-sig, expired, wrong-aud, wrong-iss, nonce-mismatch, missing-kid). 1 integration smoke (wiremock JWKS + happy-path callback).
- [ ] `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-cli --test sso_oidc_jwks_smoke` passes (if added).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-sso-oidc-jwt-signature-verify`
- PR title: `gateway: verify OIDC ID-token JWT signature via IdP JWKS (mission-sso-oidc-jwt-signature-verify)`

## Notes for the implementer

- `jsonwebtoken` is preferred (mature, popular, accepts pre-fetched JWKS via `DecodingKey::from_jwk`). `josekit` is an alternative with broader JWS/JWE support but heavier dep tree.
- JWKS fetching can reuse the existing `reqwest`-based HTTP client patterns from other gateway-side modules; do NOT introduce a second HTTP client.
- The `nonce` was minted in `/auth/login` and stored in the PKCE cookie. Confirm the field name by reading the `PkcePayload` struct in `sso_session.rs` (out-of-scope to modify, in-scope to read).
- Audit-row event-name suggestion: `sso.oidc.id_token.verified` / `sso.oidc.id_token.rejected`. Match existing audit-event naming conventions (search `audit_kind` for the prefix style).
- Operator-visible: **before this lands, an enterprise customer can't reach OIDC compliance.** Tier-1 review will scrutinize the verification path closely.
- This brief is the security-hardening companion to #1469. Once it lands plus `mission-sso-saml-xml-dsig-verify`, the SSO surface is production-ready.
