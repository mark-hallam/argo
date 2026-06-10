//! Pure-mapping + tenant-scoped resolver tests for the Genesis capability
//! fence (DEC-025 P2 + P5 (b)). Extracted from `turn_vertical_fence.rs` to
//! keep that file under the 500-LOC ceiling; declared via
//! `#[path] mod derive_tests;`.

use super::*;
use argo_vertical::{
    ActionDeclaration, ActionsSection, McpServerDeclaration, McpServersSection, ToolsSection,
    VerticalIdentity, VerticalManifest, SCHEMA_VERSION,
};
use std::collections::BTreeMap;

fn manifest_with(
    actions: &[&str],
    allowed: &[&str],
    denied: &[&str],
    mcp_servers: &[&str],
) -> VerticalManifest {
    let mut action_map: BTreeMap<String, ActionDeclaration> = BTreeMap::new();
    for a in actions {
        action_map.insert((*a).to_string(), ActionDeclaration::default());
    }
    let mut servers: BTreeMap<String, McpServerDeclaration> = BTreeMap::new();
    for s in mcp_servers {
        servers.insert(
            (*s).to_string(),
            McpServerDeclaration {
                transport: "stdio".into(),
                command: "npx".into(),
                args: vec![],
            },
        );
    }
    VerticalManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        vertical: VerticalIdentity {
            id: "demo".into(),
            name: "Demo".into(),
            version: "0.0.1".into(),
            ..VerticalIdentity::default()
        },
        actions: Some(ActionsSection {
            actions: action_map,
        }),
        tools: Some(ToolsSection {
            allowed: allowed.iter().map(|s| (*s).to_string()).collect(),
            denied: denied.iter().map(|s| (*s).to_string()).collect(),
        }),
        mcp_servers: (!servers.is_empty()).then_some(McpServersSection { servers }),
        ..VerticalManifest::default()
    }
}

#[test]
fn derive_maps_actions_and_allowed_tools() {
    let m = manifest_with(
        &["draft_reply", "summarize_ticket"],
        &["web_search"],
        &[],
        &[],
    );
    let profile = derive_profile_from_manifest(&m);
    assert!(profile.permits_tool("draft_reply"));
    assert!(profile.permits_tool("summarize_ticket"));
    assert!(profile.permits_tool("web_search"));
    assert!(!profile.permits_tool("terminal"));
}

#[test]
fn derive_keeps_mcp_servers_draft_only() {
    let m = manifest_with(&["draft_reply"], &[], &[], &["gmail", "stripe"]);
    let profile = derive_profile_from_manifest(&m);
    // Recorded as draft integrations, NOT live dispatchable tools.
    assert_eq!(profile.draft_integrations(), vec!["gmail", "stripe"]);
    assert!(!profile.permits_tool("gmail"));
    assert!(!profile.permits_tool("stripe"));
}

#[test]
fn derive_respects_deny_list() {
    let m = manifest_with(&["send_email"], &["send_email"], &["send_email"], &[]);
    let profile = derive_profile_from_manifest(&m);
    assert!(!profile.permits_tool("send_email"));
}

#[test]
fn empty_manifest_yields_deny_all_profile() {
    let m = VerticalManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        vertical: VerticalIdentity {
            id: "bare".into(),
            name: "Bare".into(),
            version: "0.0.1".into(),
            ..VerticalIdentity::default()
        },
        ..VerticalManifest::default()
    };
    let profile = derive_profile_from_manifest(&m);
    assert!(profile.allowed_tools().is_empty());
    assert!(!profile.permits_tool("anything"));
}

#[test]
fn non_genesis_worker_is_not_fenced() {
    // A session whose worker carries NO Genesis provenance keeps the
    // historical owner binding — `None` regardless of vertical id.
    // (Uses the public wrapper; `default_dir()` is irrelevant on the
    // `None` arm — it short-circuits before any disk read.)
    assert!(
        resolve_worker_profile_for_session(TenantId::new(), None, GenesisProvenance::None)
            .is_none()
    );
    assert!(resolve_worker_profile_for_session(
        TenantId::new(),
        Some("ai-receptionist"),
        GenesisProvenance::None
    )
    .is_none());
}

#[test]
fn genesis_worker_bound_to_catalog_id_is_fenced() {
    // The bypass this closes: a Genesis-provenance worker bound to a
    // CATALOG vertical id (not under `.genesis/`) MUST resolve to a
    // least-privilege profile derived from that catalog manifest — not
    // `None` (which the caller would bind to owner). Race-free: points
    // the resolver at a tempdir catalog via the root-parameterized
    // inner fn instead of mutating the process-global env var.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let vdir = tmp.path().join("intern");
    std::fs::create_dir_all(&vdir).expect("mkdir");
    std::fs::write(
        vdir.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"intern\"\nname = \"Intern\"\nversion = \"0.1.0\"\n\
         [actions.draft_reply]\nrisk = \"low\"\nrequires_approval = false\n\
         description = \"Draft a reply.\"\n\
         [tools]\nallowed = [\"web_search\"]\ndenied = []\n",
    )
    .expect("write manifest");

    let profile = resolve_worker_profile_in(
        tmp.path(),
        TenantId::new(),
        Some("intern"),
        GenesisProvenance::Genesis,
    )
    .expect("Genesis worker must always yield a profile, never None/owner");
    assert!(
        profile.permits_tool("draft_reply"),
        "declared action allowed"
    );
    assert!(profile.permits_tool("web_search"), "allowed tool allowed");
    assert!(
        !profile.permits_tool("terminal"),
        "undeclared tool must stay denied under the fence"
    );
}

#[test]
fn genesis_worker_with_unresolvable_manifest_fails_closed() {
    // A known-Genesis worker whose manifest can't be resolved (no
    // matching .genesis/ dir, not in the catalog) must NOT fall back
    // to owner — it gets a deny-all profile so the fence fails CLOSED.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let profile = resolve_worker_profile_in(
        tmp.path(),
        TenantId::new(),
        Some("a-vertical-that-does-not-exist-anywhere"),
        GenesisProvenance::Genesis,
    )
    .expect("known-Genesis session must always yield a profile, never None");
    assert!(
        profile.allowed_tools().is_empty(),
        "fail-closed profile must have an empty allow-set"
    );
    assert!(!profile.permits_tool("terminal"));
    assert!(!profile.permits_tool("send_email"));
}

#[test]
fn genesis_worker_with_no_vertical_id_fails_closed() {
    // Defensive: a Genesis worker with a missing vertical binding is
    // still fenced (deny-all), never owner.
    let profile =
        resolve_worker_profile_for_session(TenantId::new(), None, GenesisProvenance::Genesis)
            .expect("known-Genesis session must always yield a profile");
    assert!(profile.allowed_tools().is_empty());
}

#[test]
fn genesis_worker_resolves_only_its_own_tenant_dotgenesis_manifest() {
    // P5 (b): a Genesis worker's profile is derived from ITS tenant's
    // `.genesis/<tenant>/<id>/` manifest. A manifest another tenant wrote
    // under the SAME vertical_id must NOT leak into this worker's profile
    // — cross-tenant resolve is denied, so the fence falls through to the
    // deny-all fail-closed profile.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let alice = TenantId::new();
    let bob = TenantId::new();
    let alice_slug = sanitize_tenant(&alice.to_string());
    // Alice's tenant has a permissive manifest for `genesis-intern`.
    let alice_dir = tmp
        .path()
        .join(".genesis")
        .join(&alice_slug)
        .join("genesis-intern");
    std::fs::create_dir_all(&alice_dir).expect("mkdir");
    std::fs::write(
        alice_dir.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"genesis-intern\"\nname = \"Intern\"\nversion = \"0.1.0\"\n\
         [actions.draft_reply]\nrisk = \"low\"\nrequires_approval = false\n\
         description = \"Draft a reply.\"\n\
         [tools]\nallowed = [\"web_search\"]\ndenied = []\n",
    )
    .expect("write manifest");

    // Alice resolves her own manifest → permissive profile.
    let alice_profile = resolve_worker_profile_in(
        tmp.path(),
        alice,
        Some("genesis-intern"),
        GenesisProvenance::Genesis,
    )
    .expect("own-tenant Genesis worker yields a profile");
    assert!(alice_profile.permits_tool("web_search"));

    // Bob, bound to the SAME vertical_id, must NOT pick up alice's
    // manifest. With no catalog entry either, he fails closed (deny-all).
    let bob_profile = resolve_worker_profile_in(
        tmp.path(),
        bob,
        Some("genesis-intern"),
        GenesisProvenance::Genesis,
    )
    .expect("known-Genesis session always yields a profile");
    assert!(
        bob_profile.allowed_tools().is_empty(),
        "another tenant's manifest must NOT leak into this worker's fence"
    );
    assert!(!bob_profile.permits_tool("web_search"));
}

// ---- DEC-049: the production grant gate (manifest opt-in + ARGO_WORKER_SWARM) ----

#[test]
fn delegate_manifest_grants_subagent_spawn_only_when_flag_on() {
    use argo_types::{Capability, SpawnKind};
    let subagent = Capability::Spawn {
        spawn_kind: SpawnKind::Subagent,
    };
    // A manifest that opts in by allowing the delegate_task tool.
    let m = manifest_with(&[], &["delegate_task"], &[], &[]);
    let prev = std::env::var("ARGO_WORKER_SWARM").ok();

    // Flag OFF (default): the opt-in manifest still grants NO spawn.
    std::env::remove_var("ARGO_WORKER_SWARM");
    let off = derive_profile_from_manifest(&m);
    assert!(!off.can_spawn_subagent(), "flag off → no grant");
    assert!(!off.to_capability_set().covers(&subagent));

    // Flag ON + manifest opt-in → Subagent spawn granted (and nothing else).
    std::env::set_var("ARGO_WORKER_SWARM", "1");
    let on = derive_profile_from_manifest(&m);
    assert!(on.can_spawn_subagent(), "flag on + delegate_task allowed → grant");
    assert!(on.to_capability_set().covers(&subagent));
    assert!(!on.to_capability_set().covers(&Capability::Spawn {
        spawn_kind: SpawnKind::Subprocess
    }));

    // Flag ON but the manifest does NOT allow delegate_task → still no grant.
    let no_optin = manifest_with(&[], &["web_search"], &[], &[]);
    assert!(
        !derive_profile_from_manifest(&no_optin).can_spawn_subagent(),
        "no delegate_task in manifest → no grant even with the flag on"
    );

    match prev {
        Some(v) => std::env::set_var("ARGO_WORKER_SWARM", v),
        None => std::env::remove_var("ARGO_WORKER_SWARM"),
    }
}
