//! End-to-end proof of the DEC-025 Genesis capability fence at the
//! real dispatch seam.
//!
//! Proves three things the brief demands:
//!  1. a Genesis-hired teammate does NOT hold the owner's `*` keys;
//!  2. an action OUTSIDE its derived profile is DENIED at dispatch;
//!  3. the deterministic irreversibility gate forces sensitive verbs
//!     to always-human regardless of the profile.
//!
//! Exercises the production `build_dispatch_ctx` (the single seam that
//! binds the acting principal) with a derived
//! `WorkerCapabilityProfile`, then asserts both the capability-engine
//! verdict AND the actual `dispatch_one_with_runtime` result.

use crate::dispatch::{build_dispatch_ctx, dispatch_one_with_runtime};
use argo_capability::{is_irreversible_action, WorkerCapabilityProfile};
use argo_store::SessionStore;
use argo_types::{Capability, ToolCallId, ToolCallRequest};
use std::sync::Arc;

/// Build a dispatch context for a teammate fenced to `profile`.
async fn ctx_for_profile(
    cwd: &std::path::Path,
    profile: Option<&WorkerCapabilityProfile>,
) -> (
    argo_tool::ToolRegistry,
    argo_capability::CapabilityEngine,
    argo_types::Principal,
) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    let orchestrator = Arc::new(argo_orchestrator::OrchestratorStore::new(store.clone()));
    build_dispatch_ctx(cwd, None, None, orchestrator, store, profile).expect("build dispatch ctx")
}

fn tool_cap(name: &str) -> Capability {
    Capability::ToolDispatch {
        tool_name_glob: name.to_string(),
    }
}

#[tokio::test]
async fn genesis_teammate_does_not_hold_owner_star_keys() {
    let tmp = tempfile::tempdir().expect("tmpdir");
    // A teammate whose generated spec declares exactly one action.
    let profile = WorkerCapabilityProfile::derive(
        ["draft_reply".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    let (_reg, engine, principal) = ctx_for_profile(tmp.path(), Some(&profile)).await;

    // The owner role's all-access reach is GONE: the wildcard glob, the
    // broad net egress, and the all-secrets read are all denied.
    assert!(
        engine.check(&principal, &tool_cap("*")).is_err(),
        "fenced teammate must NOT hold ToolDispatch \"*\""
    );
    assert!(
        engine
            .check(
                &principal,
                &Capability::NetEgress {
                    host_glob: "**".into(),
                    port: None,
                }
            )
            .is_err(),
        "fenced teammate must NOT hold NetEgress \"**\""
    );
    assert!(
        engine
            .check(&principal, &Capability::SecretRead { name: "*".into() })
            .is_err(),
        "fenced teammate must NOT hold SecretRead \"*\""
    );
    // But its one declared action IS permitted.
    assert!(
        engine.check(&principal, &tool_cap("draft_reply")).is_ok(),
        "declared action must be permitted"
    );
}

#[tokio::test]
async fn out_of_profile_action_is_denied_at_dispatch() {
    let tmp = tempfile::tempdir().expect("tmpdir");
    // Profile grants `clarify` (via the always-safe minimal set) but NOT
    // `terminal` (a side-effecting, undeclared tool).
    let profile = WorkerCapabilityProfile::derive(
        ["draft_reply".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    let (registry, engine, principal) = ctx_for_profile(tmp.path(), Some(&profile)).await;

    // Engine verdict: terminal denied, clarify allowed.
    assert!(
        engine.check(&principal, &tool_cap("terminal")).is_err(),
        "undeclared tool must be denied by the fence"
    );
    assert!(engine.check(&principal, &tool_cap("clarify")).is_ok());

    // And the real dispatch path returns an error result for the
    // out-of-profile tool (the loop surfaces it to the model rather than
    // executing it).
    let session_id = argo_orchestrator::SessionId::new();
    let call = ToolCallRequest {
        id: ToolCallId::new(),
        provider_call_id: None,
        name: "terminal".into(),
        arguments: serde_json::json!({ "command": "echo pwned" }),
    };
    let result = dispatch_one_with_runtime(
        &registry,
        &engine,
        &principal,
        argo_types::TenantId(uuid::Uuid::from_u128(1)),
        tmp.path(),
        &session_id,
        &call,
        None,
        None,
    )
    .await;
    assert!(
        result.is_error,
        "dispatch of an out-of-profile tool must return an error result, got: {result:?}"
    );
}

#[tokio::test]
async fn non_genesis_session_keeps_owner_reach() {
    // Regression guard: a session whose worker has NO Genesis provenance
    // (a hand-authored catalog vertical the operator authored + trusts, or
    // an ad-hoc session with no worker) resolves to `profile = None` and
    // MUST keep the historical `owner` binding so existing work is
    // unchanged. This is owner-by-DESIGN — distinct from the bug below.
    let tmp = tempfile::tempdir().expect("tmpdir");
    // `None` is exactly what the provenance-keyed resolver returns for a
    // non-Genesis worker (see
    // `turn_vertical_fence::resolve_worker_profile_for_session` with
    // `GenesisProvenance::None`).
    let (_reg, engine, principal) = ctx_for_profile(tmp.path(), None).await;
    assert!(
        engine.check(&principal, &tool_cap("terminal")).is_ok(),
        "non-Genesis session keeps owner's ToolDispatch \"*\" reach"
    );
    assert!(engine
        .check(&principal, &tool_cap("any_tool_at_all"))
        .is_ok());
}

/// Write a catalog (NOT `.genesis/`) verticals root under `root` that
/// declares one action + one allowed tool for `id`. This is the shape a
/// first-hire'd Genesis worker binds to (`suggested_vertical = vertical_id`,
/// a catalog id that is never written to `.genesis/`).
fn write_catalog_vertical(root: &std::path::Path, id: &str) {
    let vdir = root.join(id);
    std::fs::create_dir_all(&vdir).expect("mkdir vertical");
    std::fs::write(
        vdir.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"Support Intern\"\nversion = \"0.1.0\"\n\
             [actions.draft_reply]\nrisk = \"low\"\nrequires_approval = false\n\
             description = \"Draft a reply to a support ticket.\"\n\
             [tools]\nallowed = [\"web_search\"]\ndenied = []\n"
        ),
    )
    .expect("write manifest");
}

/// Assert a bound principal is FENCED, not owner: the owner role's
/// `ToolDispatch "*"`, `NetEgress "**"`, and `SecretRead "*"` reach are all
/// gone, an undeclared tool is denied, and (via the real dispatch path) an
/// out-of-profile call returns an error result rather than executing.
async fn assert_principal_fully_fenced(cwd: &std::path::Path, profile: &WorkerCapabilityProfile) {
    let (registry, engine, principal) = ctx_for_profile(cwd, Some(profile)).await;
    assert!(
        engine.check(&principal, &tool_cap("*")).is_err(),
        "fenced worker must NOT hold ToolDispatch \"*\""
    );
    let net = Capability::NetEgress {
        host_glob: "**".into(),
        port: None,
    };
    assert!(
        engine.check(&principal, &net).is_err(),
        "fenced worker must NOT hold NetEgress \"**\""
    );
    assert!(
        engine
            .check(&principal, &Capability::SecretRead { name: "*".into() })
            .is_err(),
        "fenced worker must NOT hold SecretRead \"*\""
    );
    assert!(
        engine.check(&principal, &tool_cap("terminal")).is_err(),
        "undeclared tool denied for the fenced worker"
    );
    let call = ToolCallRequest {
        id: ToolCallId::new(),
        provider_call_id: None,
        name: "terminal".into(),
        arguments: serde_json::json!({ "command": "echo pwned" }),
    };
    let result = dispatch_one_with_runtime(
        &registry,
        &engine,
        &principal,
        argo_types::TenantId(uuid::Uuid::from_u128(1)),
        cwd,
        &argo_orchestrator::SessionId::new(),
        &call,
        None,
        None,
    )
    .await;
    assert!(
        result.is_error,
        "dispatch of an out-of-profile tool on a fenced worker must error, got: {result:?}"
    );
}

#[tokio::test]
async fn genesis_worker_bound_to_catalog_id_is_fenced_not_owner() {
    // THE BYPASS THIS CLOSES (adversarial-verification P0). A teammate
    // hired via `POST /api/v1/onboarding/first-hire` is a genuine Genesis
    // hire — its persona carries `genesis_plan_id=` provenance — but it is
    // bound to a CATALOG `vertical_id` that is never written to
    // `.genesis/`. The old fence keyed off directory location, so it
    // returned `None` for such a worker → the caller bound it to `owner`
    // (`*` keys). The fix keys off provenance, so a Genesis worker bound to
    // a catalog id now resolves to a FENCED least-privilege profile.
    //
    // Exercises the REAL chain: provenance(Genesis) + catalog vertical_id
    //   → resolve_worker_profile_in → build_dispatch_ctx → assert FENCED.
    // Uses the root-parameterized `resolve_worker_profile_in` (the core of
    // the production `resolve_worker_profile_for_session`) so the test
    // points at a fixture catalog without racing other tests on the
    // process-global `$ARGO_VERTICALS_DIR`.
    use crate::turn_vertical_fence::{resolve_worker_profile_in, GenesisProvenance};

    let tmp = tempfile::tempdir().expect("tmpdir");
    let verticals = tmp.path().join("verticals");
    write_catalog_vertical(&verticals, "support-intern");

    // The Scene-1 worker: Genesis provenance + a catalog vertical id.
    let profile = resolve_worker_profile_in(
        &verticals,
        argo_types::TenantId(uuid::Uuid::from_u128(1)),
        Some("support-intern"),
        GenesisProvenance::Genesis,
    )
    .expect("a Genesis-provenance worker must ALWAYS yield a fence, never None/owner");

    // The profile is the least-privilege one derived from the catalog
    // manifest — its declared action + allowed tool, and nothing else.
    assert!(
        profile.permits_tool("draft_reply"),
        "declared action allowed"
    );
    assert!(profile.permits_tool("web_search"), "allowed tool allowed");
    assert!(
        !profile.permits_tool("terminal"),
        "undeclared side-effecting tool must be denied"
    );

    // Bind it at the production dispatch seam: FENCED, not owner.
    assert_principal_fully_fenced(tmp.path(), &profile).await;
}

#[tokio::test]
async fn genesis_worker_with_unresolvable_manifest_fails_closed_not_owner() {
    // Fail-closed: a known-Genesis worker whose bound vertical can't be
    // resolved (missing / malformed / not in catalog) must get a deny-all
    // profile, NEVER the owner fallback the old `.ok()?` chain produced.
    use crate::turn_vertical_fence::{resolve_worker_profile_in, GenesisProvenance};
    let tmp = tempfile::tempdir().expect("tmpdir");
    // Empty verticals root → nothing under `.genesis/`, nothing in catalog.
    let empty_root = tmp.path().join("verticals");
    std::fs::create_dir_all(&empty_root).expect("mkdir empty root");
    let profile = resolve_worker_profile_in(
        &empty_root,
        argo_types::TenantId(uuid::Uuid::from_u128(1)),
        Some("nonexistent-vertical-xyz"),
        GenesisProvenance::Genesis,
    )
    .expect("known-Genesis worker must yield a profile (deny-all), never None");
    assert!(
        profile.allowed_tools().is_empty(),
        "fail-closed profile must have an empty allow-set"
    );
    // Same fenced-not-owner posture as the catalog case: no `*` reach, every
    // side-effecting tool denied at the real dispatch seam.
    assert_principal_fully_fenced(tmp.path(), &profile).await;
}

#[test]
fn irreversible_action_always_requires_human() {
    // The deterministic floor: these always trip the gate regardless of
    // any profile the LLM might generate.
    for verb in [
        "wire_funds",
        "issue_refund",
        "drop_table",
        "terminate_employee",
        "cancel_subscription",
    ] {
        assert!(
            is_irreversible_action(verb),
            "{verb} must be classified irreversible (always-human)"
        );
    }
    // A draft / read action is NOT forced to human by the irreversibility
    // gate (it can still be gated by the profile / safety judge).
    assert!(!is_irreversible_action("draft_reply"));
    assert!(!is_irreversible_action("summarize_ticket"));
}
