//! Demo workflow definitions inserted by the canonical seed.
//!
//! The Scene-5 combo workflow `customer_research_followup` (per
//! `mission-demo-seed-scene-5-zendesk-to-macos-notes-combo-workflow`)
//! is the only workflow the seed inserts. Its definition body is a
//! valid `[workflow].schema_version = "1"` TOML that the
//! `argo_vertical::parse_workflow` validator parses cleanly — that
//! parse is what `argo_store::workflow_definition::create` runs
//! pre-insert.
//!
//! Execution at recording time is live (Scene 5 runs the workflow
//! on camera), not pre-baked. The seed only declares the workflow
//! definition so the /workflows page renders a non-empty list.

use argo_store::workflow_definition::{self, WorkflowDefinitionError};
use argo_store::SessionStore;
use argo_types::{ArgoError, TenantId};

use crate::SeedResult;

/// Vertical id the workflow definition rides under. The Scene-5
/// canon talks about a Zendesk-driven research workflow that lands
/// notes into the macOS Notes app via computer-use, so the
/// marketplace vertical is the natural home (Mariah owns the
/// follow-up beat in the runbook).
const DEMO_VERTICAL_ID: &str = "marketplace";

/// Workflow name surfaced in the audit-row payload and the
/// `/api/v1/verticals/{vid}/workflows/{name}/...` route.
const DEMO_WORKFLOW_NAME: &str = "customer_research_followup";

/// The literal `.workflow.toml` body inserted into
/// `workflow_definition.definition_toml`. Three steps mirror Scene
/// 5's beat structure:
///   1. fetch the open Zendesk thread (`skill_invoke` →
///      `zendesk fetch thread`),
///   2. summarize the open question (`skill_invoke` →
///      `summarize for followup`),
///   3. drop a note into the macOS Notes app via computer-use
///      (`tool_call` → `cu.run_macro` against the notes target).
const DEMO_WORKFLOW_TOML: &str = r#"schema_version = "1"

[workflow]
name        = "customer_research_followup"
description = "Pull a customer's open Zendesk thread, summarise the research question, and drop a follow-up note into the macOS Notes app via computer-use."

[[step]]
name  = "fetch_thread"
kind  = "skill_invoke"
skill = "zendesk fetch thread"

[[step]]
name  = "summarize_question"
kind  = "skill_invoke"
skill = "summarize for followup"
depends_on = ["fetch_thread"]

[[step]]
name = "drop_note"
kind = "tool_call"
tool = "cu.run_macro"
args = { app = "Notes", action = "new_note", title = "Follow-up", body = "{{summarize_question.result}}" }
depends_on = ["summarize_question"]
"#;

/// Insert the canonical Scene-5 workflow definition under
/// `tenant_id`. Returns the number of workflow definitions inserted
/// (always `1` on success).
pub async fn seed_demo_workflow(
    store: &SessionStore,
    tenant_id: TenantId,
    actor: &str,
) -> SeedResult<usize> {
    match workflow_definition::create(
        store.pool(),
        tenant_id,
        DEMO_VERTICAL_ID,
        DEMO_WORKFLOW_NAME,
        DEMO_WORKFLOW_TOML,
        None,
        actor,
    )
    .await
    {
        Ok(_) => Ok(1),
        // Re-seeding the same tenant produces a UNIQUE collision on
        // the second pass — treat as a benign no-op so `argo demo
        // seed --tenant <id>` is idempotent for the operator.
        Err(WorkflowDefinitionError::UniqueViolation { .. }) => Ok(0),
        Err(WorkflowDefinitionError::Argo(e)) => Err(crate::SeedError::Argo(e)),
    }
}

/// Map a [`WorkflowDefinitionError`] to a generic [`ArgoError`] for
/// crate-internal callers that don't care about the typed branch.
#[allow(dead_code)]
fn flatten_workflow_error(e: WorkflowDefinitionError) -> ArgoError {
    match e {
        WorkflowDefinitionError::Argo(inner) => inner,
        WorkflowDefinitionError::UniqueViolation { vertical, name } => ArgoError::Provider(
            format!("workflow_definition: collision on vertical={vertical}, name={name}"),
        ),
    }
}
