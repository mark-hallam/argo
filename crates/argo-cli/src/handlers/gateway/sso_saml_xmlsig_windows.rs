//! Windows fail-closed stub for SAML XML-DSig verification.
//!
//! The real verifier (`sso_saml_xmlsig.rs`) is built on `bergshamra-dsig`
//! / `bergshamra-keys`, which transitively pull in the `kryptering`
//! PKCS#11/HSM backend. `kryptering` does not compile on Windows:
//! `CK_ULONG` is 32-bit there, so `cryptoki 0.7` exposes `From<u32>`
//! rather than the `From<u64>` the crate relies on (E0277 at
//! `kryptering/src/pkcs11/mod.rs:147` + `:610`). Those crates are therefore
//! excluded from the Windows dependency graph (see `Cargo.toml`), so
//! XML-DSig signature verification is unavailable on this target.
//!
//! Posture: **fail closed.** A SAML ACS handler that cannot
//! cryptographically verify the IdP's signature must reject the response —
//! never accept it. Accepting an unverified assertion would be an
//! authentication bypass. So `verify_saml_response` always returns
//! [`SamlXmlSigError::VerificationBackendUnavailable`] on Windows. NO
//! THEATER: this is a real capability gap surfaced as a hard rejection, not
//! a faked pass. The unix verifier — including the enterprise HSM path — is
//! unchanged.
//!
//! Tracked: `win-kryptering-cfg-gate-hsm-signing-backend`.

use chrono::{DateTime, Duration, Utc};

/// Windows subset of the unix verifier's error type
/// ([`sso_saml_xmlsig::SamlXmlSigError`] on non-Windows targets). Only the
/// single Windows-reachable variant is defined; the ACS handler
/// (`sso_saml_acs.rs`) debug-formats whichever error it receives, so it
/// compiles unchanged across targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SamlXmlSigError {
    /// XML-DSig verification is not available on this build: the signing
    /// backend (bergshamra/kryptering) does not compile on Windows, so the
    /// SAML response is rejected rather than trusted.
    VerificationBackendUnavailable,
}

/// Verified SAML assertion. Mirrors the unix type so `sso_saml_acs.rs`
/// reads the same fields on both targets. Never constructed on Windows
/// (verification always fails closed), but the fields are read by the ACS
/// handler's success branch, which is compiled on every target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SamlAssertion {
    pub name_id: Option<String>,
    pub used_sha1: bool,
}

/// Always rejects on Windows: SAML XML-DSig verification is unavailable.
///
/// Signature mirrors the unix `verify_saml_response` so the ACS handler
/// (`sso_saml_acs.rs`) compiles unchanged across targets.
pub(super) fn verify_saml_response(
    xml: &str,
    tenant: &argo_config::SsoSamlTenant,
    now: DateTime<Utc>,
    clock_skew: Duration,
    in_response_to_expected: Option<&str>,
) -> Result<SamlAssertion, SamlXmlSigError> {
    // No verification backend on this target: we cannot make a trust
    // decision, so we must reject. Inputs are intentionally unused.
    let _ = (xml, tenant, now, clock_skew, in_response_to_expected);
    Err(SamlXmlSigError::VerificationBackendUnavailable)
}

/// Minimal test-support for the Windows build. The real cert/sign helpers
/// live in `sso_saml_xmlsig_tests.rs`, which is only compiled on
/// non-Windows targets (it depends on bergshamra). Tests that exercise
/// actual signature verification are gated `#[cfg(not(target_os =
/// "windows"))]`; the AuthnRequest-generation tests in `sso_saml_tests.rs`
/// only need a certificate *string* in config (the cert is never parsed by
/// those paths), so a placeholder suffices here.
#[cfg(test)]
pub(super) mod tests {
    pub(in super::super) const TEST_CERT_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "windows-build-stub-no-xmldsig-verification\n",
        "-----END CERTIFICATE-----"
    );
}
