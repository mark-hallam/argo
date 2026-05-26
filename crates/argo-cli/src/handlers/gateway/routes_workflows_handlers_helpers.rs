//! Helper functions for `routes_workflows_handlers`.

use argo_store::workflow_definition::{
    WorkflowDefinition, WorkflowDefinitionId, WorkflowLifecycleState,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use sqlx::Row;

/// 409 emitted when two POSTs to
/// `/api/v1/verticals/{vertical}/workflows/{name}` race on the
/// SQLite `UNIQUE(tenant_id, vertical_id, name, version)` constraint
/// (evaluator round-7 P2 finding — see
/// `claude-workflow-create-unique-collision-as-409`). The body echoes
/// the colliding triple so the SPA can render
/// "{name} already exists in {vertical}" without re-fetching.
pub(super) fn collision_conflict_response(vertical: &str, name: &str) -> (&'static str, String) {
    let body = json!({
        "error": "workflow already exists",
        "vertical": vertical,
        "name": name,
    });
    ("409 Conflict", body.to_string())
}

pub(super) fn conflict_response(
    current_head: &Option<WorkflowDefinition>,
    current_version: i64,
) -> (&'static str, String) {
    let current_head_definition_toml = current_head
        .as_ref()
        .map(|d| d.definition_toml.clone())
        .unwrap_or_default();
    let body = json!({
        "kind": "conflict",
        "current_version": current_version,
        "current_head_definition_toml": current_head_definition_toml,
    });
    ("409 Conflict", body.to_string())
}

pub(super) fn definition_json(def: &WorkflowDefinition) -> Value {
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
        "substrate": substrate_from_definition_toml(&def.definition_toml),
    })
}

fn substrate_from_definition_toml(toml: &str) -> &'static str {
    if let Some(explicit) = explicit_substrate(toml) {
        return explicit;
    }
    let lower = toml.to_ascii_lowercase();
    if lower.contains("cu_action")
        || lower.contains("cu.")
        || lower.contains("cu_")
        || lower.contains("computer-use")
        || lower.contains("computer use")
        || lower.contains("macos")
        || lower.contains("macos_notes")
        || lower.contains("notes app")
    {
        return "macos";
    }
    if lower.contains("windows") {
        return "windows";
    }
    if lower.contains("linux") {
        return "linux";
    }
    if lower.contains("e2b") || lower.contains("sandbox") {
        return "e2b";
    }
    if lower.contains("browser") || lower.contains("web page") {
        return "browser";
    }
    "headless"
}

fn explicit_substrate(toml: &str) -> Option<&'static str> {
    for line in toml.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        let Some((key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key != "substrate" && key != "default_substrate" {
            continue;
        }
        let value = raw_value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_ascii_lowercase()
            .replace(['_', ' '], "-");
        match value.as_str() {
            "macos" | "mac" | "darwin" | "computer-use-macos" | "cu-macos" => {
                return Some("macos");
            }
            "linux" => return Some("linux"),
            "windows" | "win32" => return Some("windows"),
            "e2b" | "sandbox" => return Some("e2b"),
            "browser" | "web" => return Some("browser"),
            "headless" | "none" => return Some("headless"),
            _ => {}
        }
    }
    None
}

pub(super) fn decode_index_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkflowDefinition> {
    let id: String = row
        .try_get("id")
        .map_err(|e| ArgoError::Provider(format!("decode id: {e}")))?;
    let tenant_str: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("decode tenant_id: {e}")))?;
    let tenant = TenantId(
        uuid::Uuid::parse_str(&tenant_str)
            .map_err(|e| ArgoError::Provider(format!("decode tenant uuid: {e}")))?,
    );
    let vertical_id: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("decode vertical_id: {e}")))?;
    let name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("decode name: {e}")))?;
    let definition_toml: String = row
        .try_get("definition_toml")
        .map_err(|e| ArgoError::Provider(format!("decode definition_toml: {e}")))?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| ArgoError::Provider(format!("decode version: {e}")))?;
    let parent_id: Option<String> = row
        .try_get::<Option<String>, _>("parent_id")
        .map_err(|e| ArgoError::Provider(format!("decode parent_id: {e}")))?
        .filter(|s| !s.is_empty());
    let lifecycle_state_str: String = row
        .try_get("lifecycle_state")
        .map_err(|e| ArgoError::Provider(format!("decode lifecycle_state: {e}")))?;
    let created_at: String = row
        .try_get("created_at")
        .map_err(|e| ArgoError::Provider(format!("decode created_at: {e}")))?;
    let modified_by: String = row
        .try_get("modified_by")
        .map_err(|e| ArgoError::Provider(format!("decode modified_by: {e}")))?;
    Ok(WorkflowDefinition {
        id: WorkflowDefinitionId(id),
        tenant_id: tenant,
        vertical_id,
        name,
        definition_toml,
        version,
        parent_id: parent_id.map(WorkflowDefinitionId),
        lifecycle_state: WorkflowLifecycleState::parse(&lifecycle_state_str)?,
        created_at,
        modified_by,
    })
}
