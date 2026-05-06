//! `argo handoff-rules show` handler.

use super::handoff_rules_list::{open_routing_store, parse_tenant, resolve_rule, target_kind_label};
use argo_orchestrator::HandoffRoutingRule;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(tenant: &str, rule_ref: &str, json: bool) -> ArgoResult<()> {
    let store = open_routing_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let rule = resolve_rule(&store, tenant_id, rule_ref).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rule)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        print_text(&rule);
    }
    Ok(())
}

fn print_text(rule: &HandoffRoutingRule) {
    println!("id:          {}", rule.id);
    println!("name:        {}", rule.name);
    println!("priority:    {}", rule.priority);
    println!("active:      {}", rule.active);
    println!("target:      {}", target_kind_label(&rule.target));
    if let Some(needle) = &rule.when.expected_outcome_contains {
        println!("when.expected_outcome_contains: {needle}");
    }
    if !rule.when.hour_of_day_utc_in.is_empty() {
        println!("when.hour_of_day_utc_in: {:?}", rule.when.hour_of_day_utc_in);
    }
    if !rule.when.weekday_utc_in.is_empty() {
        println!("when.weekday_utc_in: {:?}", rule.when.weekday_utc_in);
    }
}
