//! Per-worker runtime loading of the operator's SYNTHESIZED truth-boundary
//! policy (wire-genesis-policy-per-worker-runtime-loading — completes Genesis
//! P5 (a)). Exercises [`load_synthesized_policy_in`] against a fixture
//! verticals tree so the acceptance criteria are pinned without touching the
//! process-global `$ARGO_VERTICALS_DIR`.

use super::*;
use argo_capability::{evaluate_claim, ChannelKind, ClaimCandidate, ClaimDisposition};

/// Write a Genesis vertical dir for `tenant` + `vertical_id` under `root`,
/// with a `vertical.toml` (required by the dir resolver) and a synthesized
/// `truth-boundary-policy.toml` gating the `financial` class as `Escalate`.
/// The policy's `tenant_id` defaults to the dir's tenant; pass `policy_tenant`
/// to forge a mismatched tenant_id for the defense-in-depth test.
fn write_vertical_with_policy(
    root: &Path,
    tenant: TenantId,
    vertical_id: &str,
    policy_tenant: TenantId,
) {
    let slug = sanitize_tenant(&tenant.to_string());
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
        dir.join(BOUNDARIES_POLICY_FILENAME),
        format!(
            "id = \"genesis-boundaries\"\n\
             tenant_id = \"{}\"\n\n\
             [[claim_classes]]\n\
             name = \"financial\"\n\
             disposition = \"Escalate\"\n\
             channels = \"AllChannels\"\n\
             escalate_to = {{ kind = \"Approver\", address = \"operator\" }}\n",
            policy_tenant.0
        ),
    )
    .expect("write policy toml");
}

#[test]
fn loads_synthesized_policy_and_financial_claim_escalates() {
    // Acceptance #1: a Genesis teammate whose synthesized policy gates the
    // financial class actually Escalates a declared financial claim at
    // runtime — loaded from disk, no env var set.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let tenant = TenantId::new();
    write_vertical_with_policy(tmp.path(), tenant, "cs-agent", tenant);

    let policy = load_synthesized_policy_in(tmp.path(), tenant, Some("cs-agent"))
        .expect("synthesized policy must load for this tenant + vertical");
    assert_eq!(policy.tenant_id, tenant);

    let verdict = evaluate_claim(
        &policy,
        ChannelKind::Chat,
        &ClaimCandidate {
            text: "Our refund balance is $4,200.".into(),
            declared_classes: vec!["financial".into()],
        },
    );
    assert_eq!(
        verdict.disposition,
        ClaimDisposition::Escalate,
        "operator's financial boundary must bite at runtime"
    );
}

#[test]
fn no_per_vertical_policy_yields_none_for_fallback() {
    // Acceptance #2: a teammate with no synthesized policy file resolves to
    // None, so the caller falls back to the conservative default (or, for a
    // non-Genesis worker, Allow-by-default). A bare vertical dir with no
    // policy file must NOT spuriously resolve.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let tenant = TenantId::new();
    let slug = sanitize_tenant(&tenant.to_string());
    let dir = tmp.path().join(".genesis").join(&slug).join("bare");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("vertical.toml"),
        "schema_version = \"1\"\n[vertical]\nid = \"bare\"\nname = \"Bare\"\nversion = \"0.1.0\"\n",
    )
    .expect("write vertical.toml");

    assert!(
        load_synthesized_policy_in(tmp.path(), tenant, Some("bare")).is_none(),
        "no policy file → None"
    );
    // Missing vertical_id also resolves to None.
    assert!(load_synthesized_policy_in(tmp.path(), tenant, None).is_none());
    // Unknown vertical id → None.
    assert!(load_synthesized_policy_in(tmp.path(), tenant, Some("nope")).is_none());
}

#[test]
fn cross_tenant_worker_never_loads_another_tenants_policy() {
    // Acceptance #3: tenant A persists a policy under A's `.genesis/<A>/` dir.
    // Tenant B, bound to the SAME vertical_id, must NOT load A's policy — the
    // dir lookup is scoped to B's own `.genesis/<B>/` segment.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let alice = TenantId::new();
    let bob = TenantId::new();
    write_vertical_with_policy(tmp.path(), alice, "shared-id", alice);

    assert!(
        load_synthesized_policy_in(tmp.path(), alice, Some("shared-id")).is_some(),
        "alice loads her own policy"
    );
    assert!(
        load_synthesized_policy_in(tmp.path(), bob, Some("shared-id")).is_none(),
        "bob must never reach alice's tenant-scoped policy dir"
    );
}

#[test]
fn policy_with_mismatched_tenant_id_is_refused() {
    // Defense-in-depth: even if a hand-edited policy file lands under tenant
    // A's dir but carries tenant B's `tenant_id`, the loader refuses it rather
    // than applying a foreign-tenant policy to A's session.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let alice = TenantId::new();
    let bob = TenantId::new();
    // Written under alice's dir, but the policy body claims bob's tenant_id.
    write_vertical_with_policy(tmp.path(), alice, "forged", bob);

    assert!(
        load_synthesized_policy_in(tmp.path(), alice, Some("forged")).is_none(),
        "a foreign tenant_id inside the file must be refused"
    );
}

#[test]
fn traversal_vertical_id_cannot_escape_tenant_segment() {
    // Security audit 2026-06-10: `genesis_vertical_dir_in` joined `vertical_id`
    // raw, and `Path::join` does NOT normalize `..`. We isolate the traversal
    // defense from the tenant_id backstop by writing the VICTIM policy with the
    // ATTACKER's own tenant_id — so if the traversal reached the file, the
    // backstop would PASS and the leak would land. The id allow-list must
    // reject the escape BEFORE the join, returning None regardless.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let attacker = TenantId::new();
    let victim = TenantId::new();
    // Victim's real dir, but the policy body carries the ATTACKER's tenant_id
    // (the worst case: the backstop alone would not catch this read).
    write_vertical_with_policy(tmp.path(), victim, "victim-vert", attacker);
    let victim_slug = sanitize_tenant(&victim.to_string());
    // Give the attacker their OWN `.genesis/<attacker_slug>/` segment (every
    // real tenant has one). This makes the test a true tripwire: if the
    // `is_safe_vertical_id` guard were dropped, `<attacker_slug>/../<victim>/`
    // would PHYSICALLY resolve on disk (the kernel only walks `..` through an
    // existing dir), the victim policy would load, its attacker-stamped
    // tenant_id would pass the backstop, and the assertion below would FAIL.
    // Without this dir the `..` hits ENOENT and returns None even unguarded —
    // the test would pass vacuously and not protect against a guard revert.
    write_vertical_with_policy(tmp.path(), attacker, "attacker-vert", attacker);

    for escape in [
        format!("../{victim_slug}/victim-vert"),
        format!("../../{victim_slug}/victim-vert"),
        "/etc".to_string(),
        "a/b".to_string(),
        "..".to_string(),
        "with\0nul".to_string(),
    ] {
        assert!(
            load_synthesized_policy_in(tmp.path(), attacker, Some(&escape)).is_none(),
            "traversal id {escape:?} must be rejected before the path join"
        );
    }
}

#[test]
fn safe_vertical_id_accepts_real_slugs_rejects_escapes() {
    // Pin the allow-list shape directly: real catalog/synthesized ids
    // (`[A-Za-z0-9_-]`) are accepted; anything that could traverse is not.
    for ok in ["cs-agent", "bookkeeping_assistant", "Vertical-1", "x"] {
        assert!(is_safe_vertical_id(ok), "{ok:?} is a real slug");
    }
    for bad in ["", "..", "../x", "a/b", "a\\b", "/abs", "a.b", "with\0nul"] {
        assert!(!is_safe_vertical_id(bad), "{bad:?} must be rejected");
    }
}
