//! JSON render helpers for `argo workflow def ...` (Stream
//! `mission-pm-workflow-rollback-cli`).
//!
//! Lives in a sibling module so `workflow_def.rs` stays under the
//! per-file LOC ceiling. Three shapes:
//!
//! * [`definition_json`] — full row envelope used by `list` /
//!   `show --json`. Mirrors the gateway's `routes_workflows_handlers`
//!   shape so consumers can switch transports without re-mapping.
//! * [`version_row_json`] — compact per-version row used by
//!   `versions --json` (omits the full TOML body).
//! * [`diff_endpoint_json`] — minimal envelope used by `diff --json`
//!   to identify each side without bloating the response.
use argo_store::workflow_definition::WorkflowDefinition;
use serde_json::{json, Value};

#[must_use]
pub fn definition_json(def: &WorkflowDefinition) -> Value {
    json!({
        "id": def.id.0,
        "tenant_id": def.tenant_id.to_string(),
        "vertical_id": def.vertical_id,
        "name": def.name,
        "version": def.version,
        "parent_id": def.parent_id.as_ref().map(|p| p.0.clone()),
        "lifecycle_state": def.lifecycle_state.as_str(),
        "created_at": def.created_at,
        "modified_by": def.modified_by,
        "definition_toml": def.definition_toml,
    })
}

#[must_use]
pub fn version_row_json(def: &WorkflowDefinition) -> Value {
    json!({
        "id": def.id.0,
        "version": def.version,
        "lifecycle_state": def.lifecycle_state.as_str(),
        "created_at": def.created_at,
        "modified_by": def.modified_by,
        "parent_id": def.parent_id.as_ref().map(|p| p.0.clone()),
    })
}

#[must_use]
pub fn diff_endpoint_json(def: &WorkflowDefinition) -> Value {
    json!({
        "id": def.id.0,
        "version": def.version,
        "lifecycle_state": def.lifecycle_state.as_str(),
        "created_at": def.created_at,
    })
}
