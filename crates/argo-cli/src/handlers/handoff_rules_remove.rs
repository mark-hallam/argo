//! `argo handoff-rules remove` handler.

use super::handoff_rules_list::{open_routing_store, parse_tenant, resolve_rule};
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct RemovedOut {
    id: String,
    name: String,
}

pub async fn run(tenant: &str, rule_ref: &str, json: bool) -> ArgoResult<()> {
    let store = open_routing_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let rule = resolve_rule(&store, tenant_id, rule_ref).await?;
    store.delete(tenant_id, rule.id).await?;
    let out = RemovedOut {
        id: rule.id.to_string(),
        name: rule.name.clone(),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&out)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!("removed rule '{}' (id {})", out.name, out.id);
    }
    Ok(())
}
