# mission-sso-saml-xml-dsig-verify — verify SAML response XML signature (XML-DSig)

**Owner**: claude. **Depends on**: mission-sso-saml-oidc-gateway merged (#1469, after rebase).
**Authored against**: origin/main `b1d78e98` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sso-saml-xml-dsig-verify`.

> SOURCE: deferred from #1469 PR-body. The SAML implementation in `crates/argo-cli/src/handlers/gateway/sso_saml.rs` parses ACS responses via **string-search extraction** (no XML parser dependency — XXE-immune but brittle), and **does not verify the XML-DSig signature** on the IdP's response. The `idp_cert_pem` is stored on `SsoSamlTenant` but unused. **Security implication**: unlike OIDC (where the token arrives via TLS back-channel from an authenticated `token_endpoint`), the SAML response arrives via the user-agent — anyone in the redirect path could substitute a malicious SAMLResponse with attacker-controlled `<saml:NameID>` and `<saml:Attribute>` rows. The string-based parser would happily extract those without authentication. Signature verification is the load-bearing trust anchor.

## Scope

Wire XML-DSig signature verification into the SAML ACS handler:

1. **Add a proper XML parser + XML-DSig crate.** The existing string-search parser stays for non-security-critical operations (e.g. extracting `<saml:NameID>` after sig-verify succeeds), but signature verification requires canonical XML processing. Candidate crates:
   - `xml-rs` + manual XML-DSig (no good Rust XML-DSig crate exists today — bring `sxd-document` + `sxd-xpath` + manual `XMLDSig` impl)
   - `libxml` (FFI binding to libxml2 — has XML-DSig support but pulls a system dep)
   - **Recommended**: a `samael`-style approach if available; if not, a minimal manual `XMLDSig` over `roxmltree` (XXE-safe by default) using `ring` for the actual RSA-SHA256/SHA1 verification.

   **Worker should evaluate the dep tree carefully** before committing. Document the choice + rationale in the PR body.

2. **Verify the response signature on every ACS callback** before extracting any claims:
   - Locate the `<ds:Signature>` element on the response (or on the assertion — both shapes exist; check the IdP's `WantAssertionsSigned` posture).
   - Extract `<ds:SignedInfo>` → canonicalize (Exclusive XML Canonicalization, `http://www.w3.org/2001/10/xml-exc-c14n#`).
   - Compute the canonicalized hash with the algorithm in `<ds:SignatureMethod>` (typically RSA-SHA256; accept SHA1 with explicit warning since some legacy IdPs).
   - Decode the `<ds:SignatureValue>` (base64) and verify against the public key in the stored `idp_cert_pem`.
   - Verify the `<ds:DigestValue>` for the signed reference matches the actual canonicalized `<saml:Response>` (or assertion) digest.

3. **Verify response semantics post-signature**:
   - `Issuer` matches the configured `idp_entity_id`.
   - `Destination` matches the configured `acs_url`.
   - `InResponseTo` matches the `AuthnRequest` ID minted at `/auth/saml/<tenant>/login` (replay protection).
   - `NotBefore` / `NotOnOrAfter` window includes wall-clock now (with ±60s skew tolerance).
   - `AudienceRestriction` includes the configured `sp_entity_id`.
   - **All 5 checks fire AFTER signature verification**; do not extract claims from an unverified document.

4. **Audit row on every ACS outcome**: similar to the OIDC follow-up — emit a structured audit event recording outcome (Verified / SignatureRejected / IssuerRejected / DestinationRejected / InResponseToRejected / Expired / AudienceRejected), `NameID`, and reason. Use the existing `argo-audit::AuditSink`.

5. **Test plan**: unit tests for the verification function with golden SAML responses (good, bad-sig, expired, wrong-issuer, wrong-destination, replay-attack, audience-mismatch, missing-signature). Integration test driving a full SP-init flow against a `samltest.id`-style mock fixture.

**Not in scope**: SAML response *encryption* (SAML EncryptedAssertion — rare in v1; out-of-scope). XML signature on the SP-side AuthnRequest (request signing is a separate hardening brief). HTTP-Artifact binding (HTTP-POST only). SP-initiated logout signing.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/sso_saml.rs` — narrowly scoped: replace the unverified claim-extraction with verify-then-extract. ≤200 modified lines.
- `crates/argo-cli/src/handlers/gateway/sso_saml_tests.rs` — narrowly scoped: ADD 7-8 verification tests. ≤300 added lines.
- `crates/argo-config/src/sections_sso.rs` — narrowly scoped: ensure `idp_cert_pem` is on `SsoSamlTenant` (likely already from #1469) + add optional `accept_sha1_signatures: bool` (default `false`; legacy IdPs only). ≤20 modified lines.
- `crates/argo-cli/Cargo.toml` — narrowly scoped: add the chosen XML/DSig deps. Verify workspace-wide before adding; pick the smallest dep tree.
- `<NEW: crates/argo-cli/tests/sso_saml_acs_smoke.rs>` (optional, ≤200 LOC) — golden-response-driven happy-path + signature-fail tests.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` — OIDC is a separate brief (`mission-sso-oidc-jwt-signature-verify`).
- `crates/argo-cli/src/handlers/gateway/sso_dispatch.rs` — dispatch contract is locked from #1469.
- `crates/argo-cli/src/handlers/gateway/sso_session.rs` — session-cookie HMAC is locked from #1469.
- The existing AuthnRequest builder in `sso_saml.rs` — request side is fine; only ACS response handling changes.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason. Especially: if the dep-tree evaluation shows no acceptable XML-DSig Rust crate exists, BLOCK and escalate — operator may need to pick between (a) a libxml2 FFI hard dep, (b) a hand-rolled minimal verifier (risky), or (c) deferring to v1.1.

## Acceptance criteria

- [ ] `verify_saml_response(xml: &str, tenant_cfg: &SsoSamlTenant, now, in_response_to_expected) -> Result<SamlAssertion, SamlError>` exists.
- [ ] All 7 typed errors covered: `SignatureRejected`, `IssuerRejected`, `DestinationRejected`, `InResponseToRejected`, `Expired`, `AudienceRejected`, `MissingSignature`.
- [ ] Exclusive XML Canonicalization (`http://www.w3.org/2001/10/xml-exc-c14n#`) is used; document the c14n impl source (crate or hand-rolled) in PR body.
- [ ] RSA-SHA256 supported; SHA1 supported only when `accept_sha1_signatures = true` and emits a warning-level audit row.
- [ ] Replay protection: `InResponseTo` matches the stored `AuthnRequest` ID; the stored request IDs are scrubbed after first use or after their lifetime expires.
- [ ] No claim extracted from an unverified document.
- [ ] Audit row emitted on every ACS outcome (success + each failure mode).
- [ ] 7+ unit tests + 1 integration smoke.
- [ ] `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-sso-saml-xml-dsig-verify`
- PR title: `gateway: verify SAML response XML-DSig signature (mission-sso-saml-xml-dsig-verify)`

## Notes for the implementer

- **Highest-stakes security brief in the queue**. The SAML response arrives via user-agent — without signature verification, ANY user can forge claims. Tier-1 review will scrutinize this closely.
- The dep-tree evaluation is genuinely hard. Rust's XML-DSig ecosystem is thin. Expect to spend ~30 min on dep evaluation before any code lands. If the chosen approach exceeds 200 LOC of hand-rolled crypto, **stop and BLOCK** — that's recovery-brief territory, not a single-shot landing.
- Canonicalization is the most error-prone part. Use a well-tested c14n crate; do NOT hand-roll Exclusive c14n. If no good crate exists, this brief should BLOCK and operator escalates.
- Replay protection: the in-memory store of pending `AuthnRequest` IDs needs eviction. Cap at 1000 entries + 1h TTL.
- Operator-visible: **before this lands, the SAML SSO surface is functionally insecure for enterprise.** Production deployments should either disable SAML in config OR run with this brief landed.
- Audit-event naming: `sso.saml.response.verified` / `sso.saml.response.rejected`.
- This brief is the security-hardening companion to #1469. Pair with `mission-sso-oidc-jwt-signature-verify` for full coverage.
