# Mission load-bearing wire — SAML XML signature verification

**Owner**: claude. **Depends on**: `mission-sso-saml-oidc-gateway` MERGED (#1469).
**Authored against**: origin/main `77ee9a59` on `2026-05-13`.

> SOURCE: PR-deferral from #1469. The SAML SP-init flow lands but signature verification is parked — `idp_cert_pem` is parsed but not consulted. Without this, a malicious IdP response could spoof identity. **Security-critical**.

## Scope

Add real XML signature verification to `argo-cli/src/handlers/gateway/sso_saml.rs`:

1. **Parse the SAML response XML** using the existing helper; locate `<ds:Signature>` element.
2. **Verify Signature** against the operator-configured `idp_cert_pem` using `xmlsec`-style canonicalization (Exclusive Canonical XML c14n) + RSA-SHA256.
3. **Refuse on signature mismatch** — return 401 with empty body (no info leak about why).
4. **Refuse on missing signature** — same 401 path (don't accept unsigned IdP responses).
5. **Optional: validate AssertionTime** against `NotBefore`/`NotOnOrAfter` windows with operator-configurable clock skew (default 60s).

Likely Rust crates: `xmlsec-rs` (if maintained) or `xml-rs` + `rsa` + `sha2` + `base64` doing canonicalization by hand. Verify crate health at claim time.

NOT in scope: SAML metadata auto-fetch; key rotation; multi-cert IdPs; encrypted assertions.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/sso_saml.rs` (≤ 490 LOC after) — add `verify_signature` call before assertion consumption
- `crates/argo-cli/src/handlers/gateway/sso_saml_xmlsig.rs` (NEW, ≤ 280 LOC) — xml-sig verification helper
- `crates/argo-cli/src/handlers/gateway/sso_saml_xmlsig_tests.rs` (NEW, sibling, ≤ 300 LOC) — 8 tests: valid-sig, invalid-sig, missing-sig, wrong-cert, expired-NotOnOrAfter, future-NotBefore, clock-skew-within-window, clock-skew-outside-window
- `crates/argo-cli/Cargo.toml` — add chosen xml-sig crate

## Files you must NOT touch

- `sso_oidc.rs` (orthogonal); session cookie code; existing SP-init flow shape

## Acceptance criteria

- [ ] Signature verified against `idp_cert_pem` before assertion consumed
- [ ] Refuse-on-mismatch returns 401 empty-body
- [ ] Refuse-on-missing-signature returns 401 empty-body
- [ ] Time-window validation with configurable clock skew
- [ ] 8 tests covering happy + 7 failure modes
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-saml-xml-signature-verify`
- PR title: `gateway: SAML XML signature verification (mission-saml-xml-signature-verify)`

## Notes

- **Use a maintained crate.** Rolling your own canonicalization is a common SAML security bug source. If `xmlsec-rs` isn't healthy, raise as a precursor brief.
- The empty-body 401 is consistent with the IP-allowlist pattern (#1463) — leak nothing about *why* the auth failed.
- The clock-skew default of 60s matches typical IdP recommendations. Larger windows (>5min) should be operator-explicit.
