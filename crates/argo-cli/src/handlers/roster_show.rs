//! `argo roster show` handler. Renders a roster (with members
//! hydrated) as JSON or a multi-line operator view.

use super::roster_list::{open_roster_store, parse_tenant, resolve_roster};
use argo_identity::Roster;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(tenant: &str, roster_ref: &str, json: bool) -> ArgoResult<()> {
    let store = open_roster_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let roster = resolve_roster(&store, tenant_id, roster_ref).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&roster)
                .map_err(|e| ArgoError::Provider(format!("serialize roster: {e}")))?
        );
    } else {
        print_text(&roster);
    }
    Ok(())
}

fn print_text(roster: &Roster) {
    println!("id:                  {}", roster.id);
    println!("tenant_id:           {}", roster.tenant_id);
    println!("name:                {}", roster.name);
    println!("policy.enabled:      {}", roster.policy.enabled);
    if let Some(daily) = roster.policy.daily_budget_usd_micros {
        println!("policy.daily_budget_usd_micros: {daily}");
    }
    if let Some(per_action) = roster.policy.max_usd_micros_per_action {
        println!("policy.max_usd_micros_per_action: {per_action}");
    }
    if let Some(cap) = roster.budget_cap_usd {
        println!("budget_cap_usd:      {cap:.2}");
    }
    if !roster.channels.is_empty() {
        println!("channels:");
        for c in &roster.channels {
            println!("  - {}: {}", c.channel, c.target);
        }
    }
    if roster.member_worker_ids.is_empty() {
        println!("members:             (none)");
    } else {
        println!("members ({}):", roster.member_worker_ids.len());
        for w in &roster.member_worker_ids {
            println!("  - {w}");
        }
    }
}
