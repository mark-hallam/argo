#![allow(deprecated)]

//! Wave-3 ⑩ per-worker policy ROUTING: prove the session-boot seam
//! ([`genesis_truth_boundary_ctx_for_worker_in`]) selects the operator's
//! SYNTHESIZED per-teammate truth-boundary policy when one is on disk, and
//! falls back to the conservative default when none is configured — keyed on
//! the server-authoritative `tenant_id` + `vertical_id`, scoped to the
//! teammate's own tenant.
//!
//! The disk loader (`load_synthesized_policy_in`) and the default-policy
//! enforcement are pinned elsewhere (`turn_vertical_fence_policy_load_tests`,
//! `turn_genesis_truth_boundary_tests`). What was previously UNTESTED is the
//! ROUTING decision itself — the `prefer-synthesized, else-default` branch the
//! production boot path runs. These tests close that gap against a fixture
//! verticals tree (no process-global `$ARGO_VERTICALS_DIR` mutation, so no
//! cross-test race).

use super::turn_helpers_truth_boundary_genesis::genesis_truth_boundary_ctx_for_worker_in;
use argo_capability::GENESIS_DEFAULT_POLICY_ID;
use argo_types::TenantId;
use std::path::Path;

/// The `policy_id` a Genesis-boundaries-synthesized policy carries. Mirrors
/// `argo_genesis::policy_synth::GENESIS_BOUNDARIES_POLICY_ID` BY VALUE — this
/// crate does not depend on `argo-genesis` (same posture as the fence's
/// `BOUNDARIES_POLICY_FILENAME` mirror). The two MUST stay byte-identical: the
/// gateway stamps this id on the synthesized policy and the routing seam loads
/// it back under it.
const GENESIS_BOUNDARIES_POLICY_ID: &str = "genesis-boundaries";

/// Write a Genesis vertical dir for `tenant` + `vertical_id` under `root`,
/// with the `vertical.toml` the dir resolver requires and a synthesized
/// `truth-boundary-policy.toml` (id = the canonical genesis-boundaries id)
/// gating the `financial` class as `Escalate`.
fn write_vertical_with_synthesized_policy(root: &Path, tenant: TenantId, vertical_id: &str) {
    let slug = sanitize_tenant_slug(&tenant.to_string());
    let dir = root.join(".genesis").join(&slug).join(vertical_id);
    std::fs::create_dir_all(&dir).expect("mkdir vertical dir");
    std::fs::write(
        dir.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{vertical_id}\"\nname = \"Gen\"\nversion = \"0.1.0\"\n"
        ),
    )
    .expect("write vertical.toml");
    std::fs::write(
        dir.join("truth-boundary-policy.toml"),
        format!(
            "id = \"{GENESIS_BOUNDARIES_POLICY_ID}\"\n\
             tenant_id = \"{}\"\n\n\
             [[claim_classes]]\n\
             name = \"financial\"\n\
             disposition = \"Escalate\"\n\
             channels = \"AllChannels\"\n\
             escalate_to = {{ kind = \"Approver\", address = \"operator\" }}\n",
            tenant.0
        ),
    )
    .expect("write policy toml");
}

/// Mirror of `argo_cli`/`turn_vertical_fence`'s tenant→slug map (this test
/// crate cannot reach the private fence copy). Kept byte-identical so the dir
/// the test writes matches the dir the loader reads.
fn sanitize_tenant_slug(tenant_id: &str) -> String {
    let cleaned: String = tenant_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "anonymous".to_string()
    } else {
        trimmed.to_string()
    }
}

#[test]
fn routing_selects_synthesized_per_teammate_policy_when_present() {
    // A teammate whose operator persisted a synthesized policy under its own
    // `.genesis/<tenant>/<vertical>/` dir gets THAT policy loaded at boot —
    // the per-teammate boundaries actually bind, not the conservative default.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let tenant = TenantId::new();
    write_vertical_with_synthesized_policy(tmp.path(), tenant, "cs-agent");

    let ctx = genesis_truth_boundary_ctx_for_worker_in(tmp.path(), tenant, Some("cs-agent"));

    assert_eq!(
        ctx.policy_id(),
        Some(GENESIS_BOUNDARIES_POLICY_ID),
        "the operator's synthesized per-teammate policy must be the one loaded"
    );
}

#[test]
fn routing_falls_back_to_safe_default_when_no_policy_configured() {
    // A teammate with NO synthesized policy file on disk must NOT regress: it
    // falls back to the conservative genesis default (never weaker, never
    // un-governed). A bare vertical dir with no policy file proves the
    // fallback fires, not a spurious load.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let tenant = TenantId::new();
    let slug = sanitize_tenant_slug(&tenant.to_string());
    let dir = tmp.path().join(".genesis").join(&slug).join("bare");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("vertical.toml"),
        "schema_version = \"1\"\n[vertical]\nid = \"bare\"\nname = \"Bare\"\nversion = \"0.1.0\"\n",
    )
    .expect("write vertical.toml");

    let ctx = genesis_truth_boundary_ctx_for_worker_in(tmp.path(), tenant, Some("bare"));

    assert_eq!(
        ctx.policy_id(),
        Some(GENESIS_DEFAULT_POLICY_ID),
        "no per-teammate policy on disk → conservative default, never un-governed"
    );
}

#[test]
fn routing_is_tenant_scoped_no_cross_tenant_policy_read() {
    // Tenant A persists a synthesized policy under A's segment. Tenant B,
    // bound to the SAME vertical_id, must NOT load A's policy — B routes to
    // the conservative default (its own segment has no file). The per-teammate
    // routing never crosses a tenant boundary.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let alice = TenantId::new();
    let bob = TenantId::new();
    write_vertical_with_synthesized_policy(tmp.path(), alice, "shared-id");

    let alice_ctx = genesis_truth_boundary_ctx_for_worker_in(tmp.path(), alice, Some("shared-id"));
    assert_eq!(
        alice_ctx.policy_id(),
        Some(GENESIS_BOUNDARIES_POLICY_ID),
        "alice loads her own synthesized policy"
    );

    let bob_ctx = genesis_truth_boundary_ctx_for_worker_in(tmp.path(), bob, Some("shared-id"));
    assert_eq!(
        bob_ctx.policy_id(),
        Some(GENESIS_DEFAULT_POLICY_ID),
        "bob must never reach alice's tenant-scoped policy — he falls back to the default"
    );
}

#[test]
fn routing_falls_back_to_default_for_missing_vertical_id() {
    // A Genesis teammate with no resolvable vertical_id still gets a governing
    // policy — the conservative default — rather than an un-governed turn.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let tenant = TenantId::new();

    let ctx = genesis_truth_boundary_ctx_for_worker_in(tmp.path(), tenant, None);

    assert_eq!(
        ctx.policy_id(),
        Some(GENESIS_DEFAULT_POLICY_ID),
        "a missing vertical_id must still bind the safe default"
    );
}
