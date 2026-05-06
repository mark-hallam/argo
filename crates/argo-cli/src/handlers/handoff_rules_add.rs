//! `argo handoff-rules add` handler. Reads a JSON file describing
//! the rule (name, when, then, priority, active) and persists it.

use super::handoff_rules_list::{open_routing_store, parse_tenant};
use argo_orchestrator::{HandoffRoutingRule, RuleId, TargetSelector, WhenClause};
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// JSON schema accepted by `argo handoff-rules add`. Mirrors the
/// brief's YAML/TOML DSL — `then` is a single target_selector
/// variant. `id` defaults to a fresh UUIDv7.
#[derive(Debug, Deserialize)]
struct RuleSpec {
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    when: WhenClause,
    then: TargetSelector,
    #[serde(default)]
    priority: u32,
    #[serde(default = "default_active")]
    active: bool,
}

fn default_active() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct AddedOut {
    id: String,
    name: String,
    priority: u32,
    active: bool,
}

pub async fn run(tenant: &str, path: &Path, json: bool) -> ArgoResult<()> {
    let store = open_routing_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let spec: RuleSpec = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let id = match spec.id {
        Some(s) => s
            .parse()
            .map_err(|e| ArgoError::Provider(format!("rule id '{s}': {e}")))?,
        None => RuleId::new(),
    };
    let rule = HandoffRoutingRule {
        id,
        name: spec.name,
        when: spec.when,
        target: spec.then,
        priority: spec.priority,
        active: spec.active,
    };
    store.create(tenant_id, &rule).await?;
    let out = AddedOut {
        id: rule.id.to_string(),
        name: rule.name.clone(),
        priority: rule.priority,
        active: rule.active,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&out)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!(
            "added rule '{}' (id {}, priority {}, active = {})",
            out.name, out.id, out.priority, out.active
        );
    }
    Ok(())
}
