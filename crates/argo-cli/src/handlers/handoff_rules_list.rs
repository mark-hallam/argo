//! `argo handoff-rules` dispatch entry point + `list` subcommand
//! (Stream `claude-handoff-routing-rules`). Other subcommands live
//! in sibling `handoff_rules_*.rs` files; shared helpers
//! (open_store / parse_tenant / resolve_rule) live here.

use crate::cli::cli_handoff_cmds::HandoffRulesCmd;
use crate::handlers::{
    handoff_rules_add, handoff_rules_remove, handoff_rules_show, handoff_rules_test,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_orchestrator::{HandoffRoutingRule, RoutingRuleStore, RuleId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct ListRow {
    id: String,
    name: String,
    priority: u32,
    active: bool,
    target_kind: String,
}

/// Top-level dispatch for `argo handoff-rules ...`.
pub async fn run(cmd: HandoffRulesCmd) -> ArgoResult<()> {
    match cmd {
        HandoffRulesCmd::List { tenant, json } => list(&tenant, json).await,
        HandoffRulesCmd::Show { tenant, rule, json } => {
            handoff_rules_show::run(&tenant, &rule, json).await
        }
        HandoffRulesCmd::Add { tenant, path, json } => {
            handoff_rules_add::run(&tenant, &path, json).await
        }
        HandoffRulesCmd::Remove { tenant, rule, json } => {
            handoff_rules_remove::run(&tenant, &rule, json).await
        }
        HandoffRulesCmd::Test {
            tenant,
            expected_outcome,
            hour,
            json,
        } => handoff_rules_test::run(&tenant, &expected_outcome, hour, json).await,
    }
}

async fn list(tenant: &str, json: bool) -> ArgoResult<()> {
    let store = open_routing_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let rules = store.list(tenant_id).await?;
    let rows: Vec<ListRow> = rules
        .iter()
        .map(|r| ListRow {
            id: r.id.to_string(),
            name: r.name.clone(),
            priority: r.priority,
            active: r.active,
            target_kind: target_kind_label(&r.target),
        })
        .collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        print_table(&rows);
    }
    Ok(())
}

fn print_table(rows: &[ListRow]) {
    if rows.is_empty() {
        println!("(no routing rules)");
        return;
    }
    println!(
        "{:<36}  {:<24}  {:>8}  {:<8}  target_kind",
        "id", "name", "priority", "active"
    );
    for row in rows {
        println!(
            "{:<36}  {:<24}  {:>8}  {:<8}  {}",
            row.id,
            truncate(&row.name, 24),
            row.priority,
            row.active,
            row.target_kind,
        );
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    if max <= 3 {
        return value.chars().take(max).collect();
    }
    let mut out: String = value.chars().take(max - 3).collect();
    out.push_str("...");
    out
}

pub(crate) fn target_kind_label(target: &argo_orchestrator::TargetSelector) -> String {
    use argo_orchestrator::TargetSelector;
    match target {
        TargetSelector::SpecificWorker(_) => "specific_worker".into(),
        TargetSelector::RoundRobin(_) => "round_robin".into(),
        TargetSelector::SkillMatch(s) => format!("skill_match({s})"),
        TargetSelector::RoleMatch(r) => format!("role_match({r})"),
    }
}

/// Open a [`RoutingRuleStore`] backed by the cwd workspace's argo
/// home.
pub(crate) async fn open_routing_store() -> ArgoResult<RoutingRuleStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(RoutingRuleStore::new(Arc::new(store)))
}

/// Parse a tenant uuid string.
pub(crate) fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}

/// Resolve a rule reference (uuid or name) to the row in the
/// given tenant.
pub(crate) async fn resolve_rule(
    store: &RoutingRuleStore,
    tenant: TenantId,
    rule_ref: &str,
) -> ArgoResult<HandoffRoutingRule> {
    if let Ok(id) = rule_ref.parse::<RuleId>() {
        if let Some(r) = store.get(tenant, id).await? {
            return Ok(r);
        }
    }
    if let Some(r) = store.get_by_name(tenant, rule_ref).await? {
        return Ok(r);
    }
    Err(ArgoError::Provider(format!(
        "no routing rule matching '{rule_ref}' in tenant '{tenant}'"
    )))
}
