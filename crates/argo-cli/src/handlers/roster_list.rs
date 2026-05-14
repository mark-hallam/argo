//! `argo roster` dispatch entry point + `list` subcommand
//! (Stream `claude-worker-roster`). Other subcommands live in
//! sibling `roster_*.rs` files; this one also holds the shared
//! helpers (open_roster_store / parse_tenant / parse_roster_ref).

use crate::cli::cli_roster_cmds::RosterCmd;
use crate::handlers::{
    roster_add_member, roster_create, roster_remove_member, roster_show,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{Roster, RosterId, RosterStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct ListRow {
    id: String,
    name: String,
    member_count: usize,
    policy_enabled: bool,
    daily_budget_usd_micros: Option<u64>,
    budget_cap_usd: Option<f64>,
}

/// Top-level dispatch for `argo roster ...`.
pub async fn run(cmd: RosterCmd) -> ArgoResult<()> {
    match cmd {
        RosterCmd::Create {
            tenant,
            name,
            policy,
            channels,
            budget_cap_usd,
            json,
        } => roster_create::run(&tenant, &name, policy, channels, budget_cap_usd, json).await,
        RosterCmd::List { tenant, json } => list(&tenant, json).await,
        RosterCmd::Show { tenant, roster, json } => {
            roster_show::run(&tenant, &roster, json).await
        }
        RosterCmd::AddMember {
            tenant,
            roster,
            worker,
            json,
        } => roster_add_member::run(&tenant, &roster, &worker, json).await,
        RosterCmd::RemoveMember {
            tenant,
            roster,
            worker,
            json,
        } => roster_remove_member::run(&tenant, &roster, &worker, json).await,
    }
}

async fn list(tenant: &str, json: bool) -> ArgoResult<()> {
    let store = open_roster_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let rosters = store.list(tenant_id).await?;
    let rows: Vec<ListRow> = rosters
        .iter()
        .map(|r| ListRow {
            id: r.id.to_string(),
            name: r.name.clone(),
            member_count: r.member_worker_ids.len(),
            policy_enabled: r.policy.enabled,
            daily_budget_usd_micros: r.policy.daily_budget_usd_micros,
            budget_cap_usd: r.budget_cap_usd,
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
        println!("(no rosters)");
        return;
    }
    println!(
        "{:<36}  {:<24}  {:>7}  {:<8}  daily_budget_usd_micros  budget_cap_usd",
        "id", "name", "members", "enabled"
    );
    for row in rows {
        println!(
            "{:<36}  {:<24}  {:>7}  {:<8}  {:<24}  {}",
            row.id,
            truncate(&row.name, 24),
            row.member_count,
            row.policy_enabled,
            row.daily_budget_usd_micros
                .map(|m| m.to_string())
                .unwrap_or_else(|| "—".into()),
            row.budget_cap_usd
                .map(|b| format!("{b:.2}"))
                .unwrap_or_else(|| "—".into()),
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

/// Open a [`RosterStore`] backed by the cwd workspace's argo home.
pub(crate) async fn open_roster_store() -> ArgoResult<RosterStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(RosterStore::new(Arc::new(store)))
}

/// Parse a tenant uuid string into a [`TenantId`].
pub(crate) fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}

/// Resolve a roster reference (uuid or human-friendly name) to the
/// row in the given tenant. Errors if neither lookup hits.
pub(crate) async fn resolve_roster(
    store: &RosterStore,
    tenant: TenantId,
    roster_ref: &str,
) -> ArgoResult<Roster> {
    if let Ok(id) = roster_ref.parse::<RosterId>() {
        if let Some(r) = store.get(tenant, id).await? {
            return Ok(r);
        }
    }
    if let Some(r) = store.get_by_name(tenant, roster_ref).await? {
        return Ok(r);
    }
    Err(ArgoError::Provider(format!(
        "no roster matching '{roster_ref}' in tenant '{tenant}'"
    )))
}
