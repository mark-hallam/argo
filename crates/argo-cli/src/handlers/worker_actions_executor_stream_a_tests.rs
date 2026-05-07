//! Stream A (#A-executor-trait) coverage for the executor trait
//! shape, manifest back-compat, tagged-enum parsing, and the
//! three placeholder impls (`McpExecutor`, `ComputerUseExecutor`,
//! `ApiExecutor`) that fail cleanly until Streams B / C / H wire
//! each.
//!
//! Loaded as a sibling test module from
//! `worker_actions_executor.rs` to keep per-file LOC under the
//! 500-line budget.

use super::*;
use super::worker_actions_executor_tests::{human_bundle, rec};

#[test]
fn human_executor_kind_returns_human() {
    let exec = HumanExecutor::without_manifest();
    assert_eq!(exec.kind(), ExecutorKind::Human);
}

#[test]
fn human_executor_dispatch_returns_human_handoff_outcome() {
    // Sanity: `HumanHandoff` is the only outcome HumanExecutor
    // can produce. The dispatch shim relies on this to keep the
    // ledger row at `executing` until the operator completes.
    let r = rec("act-handoff", "verify_price", false);
    let cwd = tempfile::tempdir().expect("tempdir");
    let outcome = HumanExecutor::without_manifest()
        .dispatch(&r, cwd.path())
        .expect("dispatch");
    let _ = human_bundle(outcome);
}

#[test]
fn manifest_back_compat_legacy_string_executor_parses() {
    // `executor = "human"` (the historical form) must continue
    // to parse — every existing vertical depends on it.
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.verify_price]
risk = "low"
requires_approval = false
description = "x"
executor = "human"
"#,
    )
    .expect("legacy string form must parse");
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("verify_price"))
        .expect("verify_price");
    assert_eq!(decl.executor.kind_str(), "human");
    // Legacy form carries no optional config.
    assert_eq!(decl.executor.profile(), None);
    assert_eq!(decl.executor.budget_usd(), None);
    assert_eq!(decl.executor.mcp_server(), None);
    assert_eq!(decl.executor.api_binding(), None);
}

#[test]
fn manifest_default_executor_is_human() {
    // Omitted `executor = ...` field defaults to `Legacy("human")`.
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.verify_price]
risk = "low"
requires_approval = false
description = "x"
"#,
    )
    .expect("default form must parse");
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("verify_price"))
        .expect("verify_price");
    assert_eq!(decl.executor.kind_str(), "human");
}

#[test]
fn manifest_tagged_enum_computer_use_parses_with_profile_and_budget() {
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.post_listing]
risk = "medium"
requires_approval = true
description = "x"
executor = { kind = "computer_use", profile = "marketplace_browser", budget_usd = 0.50 }
"#,
    )
    .expect("tagged form must parse");
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("post_listing"))
        .expect("post_listing");
    assert_eq!(decl.executor.kind_str(), "computer_use");
    assert_eq!(decl.executor.profile(), Some("marketplace_browser"));
    assert_eq!(decl.executor.budget_usd(), Some(0.50));
}

#[test]
fn manifest_tagged_enum_mcp_parses_with_server_and_tool() {
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.send_followup]
risk = "low"
requires_approval = false
description = "x"
executor = { kind = "mcp", server = "gmail", tool = "drafts.send" }
"#,
    )
    .expect("tagged mcp form must parse");
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("send_followup"))
        .expect("send_followup");
    assert_eq!(decl.executor.kind_str(), "mcp");
    assert_eq!(decl.executor.mcp_server(), Some("gmail"));
    assert_eq!(decl.executor.mcp_tool(), Some("drafts.send"));
    assert_eq!(decl.executor.budget_usd(), None);
}

#[test]
fn manifest_tagged_enum_api_parses_with_binding() {
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.lookup_contact]
risk = "low"
requires_approval = false
description = "x"
executor = { kind = "api", binding = "hubspot.contacts.search" }
"#,
    )
    .expect("tagged api form must parse");
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("lookup_contact"))
        .expect("lookup_contact");
    assert_eq!(decl.executor.kind_str(), "api");
    assert_eq!(decl.executor.api_binding(), Some("hubspot.contacts.search"));
}

#[test]
fn resolve_executor_dispatches_each_kind_through_tagged_form() {
    // All four kinds should resolve to a `Box<dyn Executor>` via
    // the tagged form. Three of them fail at dispatch() time but
    // resolve() itself must succeed cleanly.
    for (kind_str, want_kind) in [
        ("human", ExecutorKind::Human),
        ("mcp", ExecutorKind::Mcp),
        ("computer_use", ExecutorKind::ComputerUse),
        ("api", ExecutorKind::Api),
        ("filesystem", ExecutorKind::Filesystem),
        ("code_execution", ExecutorKind::CodeExecution),
    ] {
        let toml = format!(
            r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.do_thing]
risk = "low"
requires_approval = false
description = "x"
executor = {{ kind = "{kind_str}" }}
"#
        );
        let manifest = argo_vertical::parse(&toml).expect("manifest parses");
        let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
        assert_eq!(exec.kind(), want_kind, "kind mismatch for '{kind_str}'");
    }
}

#[test]
fn mcp_dispatch_outside_tokio_runtime_returns_clear_error() {
    // `mission-wire-mcp-action-executor` swapped the Stream A stub
    // for a real impl that pools MCP-server connections via tokio.
    // When dispatch is called from a non-async context, the executor
    // surfaces a clear runtime-needed error instead of panicking.
    // This documents that the executor's `dispatch_blocking` requires
    // an active tokio runtime (which the production CLI path always
    // has, via `#[tokio::main]`).
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[mcp_servers]
gmail = { transport = "stdio", command = "/bin/false", args = [] }

[actions.send_email]
risk = "medium"
requires_approval = true
description = "x"
executor = { kind = "mcp", server = "gmail", tool = "drafts.send" }
"#,
    )
    .expect("manifest parses");
    let exec = resolve_executor(&manifest, "send_email").expect("resolves");
    let r = rec("act-mcp-1", "send_email", true);
    let cwd = tempfile::tempdir().expect("tempdir");
    let err = exec
        .dispatch(&r, cwd.path())
        .expect_err("dispatch outside tokio runtime must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("tokio runtime"),
        "expected runtime error, got: {msg}"
    );
}

#[test]
fn computer_use_dispatch_surfaces_missing_env_when_unconfigured() {
    // Stream B `#B-computer-use-executor` made `ComputerUseExecutor`
    // a real impl that delegates to `argo-executor-computer-use`.
    // Without `E2B_API_KEY` / `ANTHROPIC_API_KEY` in the env it
    // returns a clear "not configured" error pointing at the
    // env vars (rather than the old "not yet implemented" stub
    // text Stream A shipped). This test covers the
    // unconfigured-workspace path.
    let prev_e2b = std::env::var("E2B_API_KEY").ok();
    std::env::remove_var("E2B_API_KEY");
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.post_listing]
risk = "high"
requires_approval = true
description = "x"
executor = { kind = "computer_use", profile = "marketplace_browser" }
"#,
    )
    .expect("manifest parses");
    let exec = resolve_executor(&manifest, "post_listing").expect("resolves");
    let r = rec("act-cu-1", "post_listing", true);
    let cwd = tempfile::tempdir().expect("tempdir");
    let err = exec
        .dispatch(&r, cwd.path())
        .expect_err("computer_use without env keys must fail at dispatch time");
    let msg = err.to_string();
    if let Some(v) = prev_e2b {
        std::env::set_var("E2B_API_KEY", v);
    }
    assert!(msg.contains("not configured"), "got: {msg}");
    assert!(msg.contains("E2B_API_KEY"), "got: {msg}");
    assert!(
        msg.contains("ANTHROPIC_API_KEY"),
        "expected env-var instruction: {msg}"
    );
}

#[test]
fn api_stub_dispatch_returns_useful_not_implemented_error() {
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.lookup_contact]
risk = "low"
requires_approval = false
description = "x"
executor = { kind = "api", binding = "hubspot.contacts.search" }
"#,
    )
    .expect("manifest parses");
    let exec = resolve_executor(&manifest, "lookup_contact").expect("resolves");
    let r = rec("act-api-1", "lookup_contact", false);
    let cwd = tempfile::tempdir().expect("tempdir");
    let err = exec
        .dispatch(&r, cwd.path())
        .expect_err("api stub must fail at dispatch time");
    let msg = err.to_string();
    assert!(msg.contains("not yet implemented"), "got: {msg}");
    assert!(msg.contains("api"), "got: {msg}");
}
