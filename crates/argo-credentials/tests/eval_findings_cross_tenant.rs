//! FINDINGS-ONLY tests for `eval-credentials-cross-tenant-isolation`
//! (2026-05-23). These compile against the current API and are
//! `#[ignore]`'d so they do NOT fail the workspace suite — they exist
//! as **executable red flags** that flip back to PASS automatically
//! once the fix-briefs land the API changes.
//!
//! Each `#[ignore]` body asserts something the audit found WRONG. When
//! a fix-brief changes the API in the documented direction, the test
//! body will fail to compile (good — that is the trigger to drop the
//! finding) or its assertion will flip.
//!
//! Findings codified:
//!
//! 1. **§7 P1** — `decrypt_value` returns a raw `String`, not a
//!    `secrecy::SecretString`. Plaintext credentials flow out of the
//!    crate without a redacting newtype. Removed once the API switches
//!    to `SecretString`.
//!
//! 2. **§3 P0 + §4 P0** — `MasterKey::resolve` is tenant-blind. There
//!    is no `MasterKey::resolve_for_tenant(TenantId)` (or any
//!    per-tenant key-derivation primitive). Every credential value in
//!    the crate-owned file is wrapped with the same key. Removed once
//!    the per-tenant-key brief lands.
//!
//! The report at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-credentials-cross-tenant-isolation.md`
//! is the canonical write-up; this file is the executable companion.

use argo_credentials::{decrypt_value, encrypt_value, MasterKey};

/// §7 P1 — decrypt boundary returns raw `String`.
///
/// The assertion is structural: we observe `decrypt_value(&str, &MasterKey) -> ArgoResult<String>`,
/// and confirm the output is a plain `String` (no `SecretString` newtype).
///
/// **When this fix-brief lands**
/// (`mission-credentials-secret-newtype-at-decrypt`), `decrypt_value`
/// will return `ArgoResult<secrecy::SecretString>` and this test's
/// `let _: String = ...` binding will fail to compile — that is the
/// signal to delete this finding.
#[test]
#[ignore = "FINDINGS-ONLY: codifies §7 P1 — decrypt returns raw String, not SecretString"]
fn decrypt_returns_raw_string_not_secret_newtype() {
    let key = MasterKey::generate();
    let blob = encrypt_value("hunter2", &key).expect("encrypt");
    let plaintext: String = decrypt_value(&blob, &key).expect("decrypt");
    // The audit finding: `plaintext` is a raw `String` and can land in
    // log lines, panic messages, and debug-format dumps unredacted.
    // Once the fix lands, this `let _: String = ...` becomes a
    // compile error — drop the test then.
    let _: String = plaintext;
}

/// §3 P0 + §4 P0 — `MasterKey::resolve` is tenant-blind.
///
/// We assert the resolver signature does NOT take a `TenantId`. The
/// current signature is `resolve(home_override: Option<&Path>) -> ArgoResult<Self>`.
/// When per-tenant key derivation lands
/// (`mission-credentials-per-tenant-master-key`), a new
/// `resolve_for_tenant(TenantId)` (or equivalent) constructor will
/// exist, and this test should be deleted.
///
/// We can't statically assert "no overload takes TenantId" without
/// reflection, but we can pin the current call shape: the resolver
/// works without any tenant context.
#[test]
#[ignore = "FINDINGS-ONLY: codifies §3 P0 + §4 P0 — master-key resolution is tenant-blind"]
fn master_key_resolve_does_not_take_tenant_id() {
    // No tenant id, no per-tenant derivation. Two distinct tenants
    // would resolve the EXACT same MasterKey out of the same process
    // env / `~/.skycot/.master_key` file.
    //
    // We use a tempdir as the `home_override` so the test doesn't touch
    // the real `~/.skycot/.master_key` file. `resolve` will error with
    // "no key configured" — which is the audit's evidence: there is no
    // path here that takes a TenantId and would resolve differently per
    // tenant.
    let tmp_home = tempfile::tempdir().expect("tempdir");
    // Save & clear the env var so the test is hermetic regardless of
    // the developer's shell state. (The `master_key.rs:24` constant.)
    let prior = std::env::var(argo_credentials::MASTER_KEY_ENV).ok();
    std::env::remove_var(argo_credentials::MASTER_KEY_ENV);

    let err =
        MasterKey::resolve(Some(tmp_home.path())).expect_err("no key configured in tempdir home");

    // The error mentions the env var, not any tenant context — that is
    // the codified finding: resolution has no tenant axis.
    let msg = format!("{err}");
    assert!(
        msg.contains(argo_credentials::MASTER_KEY_ENV),
        "resolution error names the env var (tenant-blind contract): {msg}"
    );
    assert!(
        !msg.contains("tenant") && !msg.contains("Tenant"),
        "resolution error MUST NOT mention tenant — that would mean the API has been fixed and this test should be deleted: {msg}"
    );

    if let Some(v) = prior {
        std::env::set_var(argo_credentials::MASTER_KEY_ENV, v);
    }
}
