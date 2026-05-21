//! Unit tests for the `argo workflow def ...` handler suite (Stream
//! `mission-pm-workflow-rollback-cli`). Covers the 8 acceptance
//! scenarios from the brief:
//!
//! a. `list` shows the current head row.
//! b. `versions` shows newest-first.
//! c. `show` without `--version` returns head.
//! d. `show --version <id>` returns that version's body.
//! e. `rollback --yes` flips the head pointer + emits audit row.
//! f. `rollback` without `--yes` aborts via the prompt sentinel
//!    (stdin is closed in the test process, so the prompt declines).
//! g. `diff` between two versions returns a meaningful unified diff.
//! h. `archive --yes` flips `lifecycle_state` to archived + emits
//!    audit row.
//!
//! Plus a deny-path test: `workflow_read_capability` is required for
//! reads; principals bound to the `guest` role can `list` (guest has
//! `workflow.read`) but cannot `rollback` (no `workflow.edit`).
#![cfg(test)]

use super::*;
use crate::handlers::workflow_def_query::list_workflow_heads;
use argo_capability::CapabilityEngine;
use argo_store::workflow_definition as wd;
use argo_store::SessionStore;
use argo_types::{ArgoError, Capability, Principal, TenantId};
use std::path::Path;
use std::sync::Arc;

const VERTICAL: &str = "pr_checker";
const WORKFLOW: &str = "open_pr_review";

const TOML_V1: &str = r#"
schema_version = "1"

[workflow]
name = "open_pr_review"

[[step]]
name = "review"
kind = "skill_invoke"
skill = "reviewer"
"#;

const TOML_V2: &str = r#"
schema_version = "1"

[workflow]
name = "open_pr_review"

[[step]]
name = "review"
kind = "skill_invoke"
skill = "reviewer"

[[step]]
name = "summarize"
kind = "skill_invoke"
skill = "summarizer"
depends_on = ["review"]
"#;

const TOML_V3: &str = r#"
schema_version = "1"

[workflow]
name = "open_pr_review"

[[step]]
name = "review"
kind = "skill_invoke"
skill = "reviewer-v2"

[[step]]
name = "summarize"
kind = "skill_invoke"
skill = "summarizer"
depends_on = ["review"]
"#;

async fn fixture_ctx(role: &str) -> WorkflowDefContext {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let principal = Principal::Local {
        os_user: format!("test-{role}"),
    };
    let engine = CapabilityEngine::with_workspace_defaults(Path::new("."));
    engine.bind(principal.clone(), role.into()).unwrap();
    WorkflowDefContext {
        store: Arc::new(store),
        tenant,
        principal,
        engine,
        actor: format!("test-{role}"),
    }
}

async fn seed_three_versions(ctx: &WorkflowDefContext) -> (String, String, String) {
    let id1 = wd::create(
        ctx.store.pool(),
        ctx.tenant,
        VERTICAL,
        WORKFLOW,
        TOML_V1,
        None,
        "seed",
    )
    .await
    .unwrap();
    let id2 = wd::create(
        ctx.store.pool(),
        ctx.tenant,
        VERTICAL,
        WORKFLOW,
        TOML_V2,
        Some(id1.clone()),
        "seed",
    )
    .await
    .unwrap();
    let id3 = wd::create(
        ctx.store.pool(),
        ctx.tenant,
        VERTICAL,
        WORKFLOW,
        TOML_V3,
        Some(id2.clone()),
        "seed",
    )
    .await
    .unwrap();
    // Promote v3 to head so list/show have a baseline.
    wd::set_head(ctx.store.pool(), ctx.tenant, VERTICAL, WORKFLOW, &id3)
        .await
        .unwrap();
    (id1.0, id2.0, id3.0)
}

async fn count_audit_rows(ctx: &WorkflowDefContext, kind_label: &str) -> i64 {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_record WHERE tenant_id = ? AND kind = ?",
    )
    .bind(ctx.tenant.to_string())
    .bind(kind_label)
    .fetch_one(ctx.store.pool())
    .await
    .unwrap();
    row.0
}

// (a) `list` shows the current head row.
#[tokio::test]
async fn list_shows_current_head() {
    let ctx = fixture_ctx("owner").await;
    let (_, _, id3) = seed_three_versions(&ctx).await;
    let rows = list_workflow_heads(ctx.store.pool(), ctx.tenant, VERTICAL)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let head = &rows[0];
    assert_eq!(head.id.as_str(), id3);
    assert_eq!(head.name, WORKFLOW);
    // The store invariant for `set_head` flips the row to active.
    assert_eq!(
        head.lifecycle_state.as_str(),
        wd::WorkflowLifecycleState::Active.as_str()
    );
    // public handler entrypoint compiles and dispatches without panic.
    super::list(&ctx, VERTICAL, true).await.unwrap();
}

// (b) `versions` shows newest-first.
#[tokio::test]
async fn versions_returns_newest_first() {
    let ctx = fixture_ctx("owner").await;
    let (_, id2, id3) = seed_three_versions(&ctx).await;
    let rows = wd::list_versions(ctx.store.pool(), ctx.tenant, VERTICAL, WORKFLOW)
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows[0].version > rows[1].version);
    assert_eq!(rows[0].id.as_str(), id3);
    assert_eq!(rows[1].id.as_str(), id2);
    super::versions(&ctx, VERTICAL, WORKFLOW, true).await.unwrap();
}

// (c) `show` without --version returns head.
#[tokio::test]
async fn show_without_version_returns_head() {
    let ctx = fixture_ctx("owner").await;
    let (_, _, id3) = seed_three_versions(&ctx).await;
    let head = wd::get_head(ctx.store.pool(), ctx.tenant, VERTICAL, WORKFLOW)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head.id.as_str(), id3);
    assert!(head.definition_toml.contains("reviewer-v2"));
    super::show(&ctx, VERTICAL, WORKFLOW, None, true).await.unwrap();
}

// (d) `show --version <id>` returns that version.
#[tokio::test]
async fn show_with_version_returns_named_row() {
    let ctx = fixture_ctx("owner").await;
    let (id1, _, _) = seed_three_versions(&ctx).await;
    super::show(&ctx, VERTICAL, WORKFLOW, Some(&id1), true)
        .await
        .unwrap();
    // Strict load also refuses cross-name access.
    let cross_err = super::show(&ctx, VERTICAL, "different_name", Some(&id1), true)
        .await
        .unwrap_err();
    assert!(matches!(cross_err, ArgoError::NotFound(_)));
}

// (e) `rollback --yes` flips head and writes a `WorkflowHeadChanged`
//     audit row.
#[tokio::test]
async fn rollback_flips_head_and_emits_audit() {
    let ctx = fixture_ctx("owner").await;
    let (id1, _, _) = seed_three_versions(&ctx).await;
    let before = count_audit_rows(&ctx, "workflow_head_changed").await;

    super::rollback(&ctx, VERTICAL, WORKFLOW, &id1, true, true)
        .await
        .unwrap();

    let head_after = wd::get_head(ctx.store.pool(), ctx.tenant, VERTICAL, WORKFLOW)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head_after.id.as_str(), id1);
    let after = count_audit_rows(&ctx, "workflow_head_changed").await;
    assert_eq!(after, before + 1, "rollback should emit one audit row");
}

// (f) `rollback` without --yes prompts; with stdin closed in tests the
//     `confirm_or_abort` helper returns false → no mutation.
#[tokio::test]
async fn rollback_without_yes_aborts_on_closed_stdin() {
    let ctx = fixture_ctx("owner").await;
    let (id1, _, id3_before) = seed_three_versions(&ctx).await;
    // Sanity: head is currently v3.
    let pre = wd::get_head(ctx.store.pool(), ctx.tenant, VERTICAL, WORKFLOW)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pre.id.as_str(), id3_before);

    // yes=false; the test process has stdin redirected (not a TTY),
    // so confirm_or_abort returns false → handler aborts.
    super::rollback(&ctx, VERTICAL, WORKFLOW, &id1, false, true)
        .await
        .unwrap();

    let post = wd::get_head(ctx.store.pool(), ctx.tenant, VERTICAL, WORKFLOW)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        post.id.as_str(),
        id3_before,
        "head must NOT change when prompt was declined"
    );
}

// (g) `diff` between two versions emits meaningful added/removed lines.
#[tokio::test]
async fn diff_produces_meaningful_added_and_removed_lines() {
    let ctx = fixture_ctx("owner").await;
    let (id1, _, id3) = seed_three_versions(&ctx).await;
    super::diff(&ctx, VERTICAL, WORKFLOW, &id1, &id3, true)
        .await
        .unwrap();
    let a_toml = wd::get(ctx.store.pool(), &wd::WorkflowDefinitionId(id1.clone()))
        .await
        .unwrap()
        .unwrap()
        .definition_toml;
    let b_toml = wd::get(ctx.store.pool(), &wd::WorkflowDefinitionId(id3.clone()))
        .await
        .unwrap()
        .unwrap()
        .definition_toml;
    let summary = crate::handlers::workflow_def_diff::compute_line_diff(&a_toml, &b_toml);
    assert!(!summary.added.is_empty(), "v3 has tasks v1 does not");
    assert!(summary.changed > 0);
    let unified =
        crate::handlers::workflow_def_diff::format_unified_diff(&a_toml, &b_toml);
    assert!(unified.contains("+ "));
}

// (h) `archive --yes` flips lifecycle_state and emits the
//     `WorkflowVersionArchived` audit row.
#[tokio::test]
async fn archive_flips_lifecycle_state_and_emits_audit() {
    let ctx = fixture_ctx("owner").await;
    let (id1, _, _) = seed_three_versions(&ctx).await;
    let before = count_audit_rows(&ctx, "workflow_version_archived").await;

    super::archive(&ctx, VERTICAL, WORKFLOW, &id1, true, true)
        .await
        .unwrap();

    let row = wd::get(ctx.store.pool(), &wd::WorkflowDefinitionId(id1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.lifecycle_state.as_str(),
        wd::WorkflowLifecycleState::Archived.as_str()
    );
    let after = count_audit_rows(&ctx, "workflow_version_archived").await;
    assert_eq!(after, before + 1);
}

// Capability deny: a `guest`-bound principal can read but not edit.
#[tokio::test]
async fn guest_role_denies_edit_but_allows_read() {
    let ctx = fixture_ctx("guest").await;
    let (id1, _, _) = seed_three_versions(&ctx).await;

    // Read is granted to guest by workspace defaults.
    super::list(&ctx, VERTICAL, true).await.unwrap();
    super::versions(&ctx, VERTICAL, WORKFLOW, true).await.unwrap();

    // Edit is not.
    let err = super::rollback(&ctx, VERTICAL, WORKFLOW, &id1, true, true)
        .await
        .unwrap_err();
    match err {
        ArgoError::RbacDenied { role, cap } => {
            assert_eq!(role, "guest");
            assert_eq!(
                cap,
                Capability::ToolDispatch {
                    tool_name_glob: argo_capability::WORKFLOW_EDIT.to_string(),
                }
            );
        }
        other => panic!("expected RbacDenied for workflow.edit, got {other:?}"),
    }
}
