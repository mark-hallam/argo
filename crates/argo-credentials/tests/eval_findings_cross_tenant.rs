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
//! ~~2. §3 P0 + §4 P0 — `MasterKey::resolve` is tenant-blind.~~
//! FIXED 2026-05-23 by `mission-credentials-per-tenant-master-key`
//! (this brief). The new constructors
//! `MasterKey::resolve_for_tenant(tenant_id) -> Arc<MasterKey>` and
//! `MasterKey::derive_for_tenant(&WorkspaceRootKey, tenant_id)` are
//! the per-tenant entry points; the legacy tenant-blind
//! `MasterKey::resolve` remains for the workspace-file flow (which
//! has no tenant axis) and as the lazy-migration fallback for
//! decrypting blobs wrapped under the old global key. The
//! corresponding finding test was deleted with this comment as its
//! grave-marker.
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

// §3 P0 + §4 P0 finding (`master_key_resolve_does_not_take_tenant_id`)
// deleted 2026-05-23 by `mission-credentials-per-tenant-master-key`.
// See the module docstring above for the closure marker. The replacement
// per-tenant API is exercised by the sibling tests in
// `crates/argo-credentials/src/master_key_tests.rs`
// (`derive_for_two_tenants_yields_distinct_keys` +
// `decrypt_cross_tenant_fails_loudly`).
