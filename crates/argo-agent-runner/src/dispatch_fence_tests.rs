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
async fn unfenced_session_keeps_owner_reach() {
    // Regression guard: a non-Genesis session (profile = None) must keep
    // the historical `owner` binding so ad-hoc + catalog-vertical work is
    // unchanged.
    let tmp = tempfile::tempdir().expect("tmpdir");
    let (_reg, engine, principal) = ctx_for_profile(tmp.path(), None).await;
    assert!(
        engine.check(&principal, &tool_cap("terminal")).is_ok(),
        "unfenced session keeps owner's ToolDispatch \"*\" reach"
    );
    assert!(engine
        .check(&principal, &tool_cap("any_tool_at_all"))
        .is_ok());
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
