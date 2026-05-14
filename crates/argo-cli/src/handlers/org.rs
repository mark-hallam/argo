//! `argo org` handlers (V1.3 — `claude-org-awareness`). Wires the
//! 4 subcommands (tree / member / reports-to / ingest) onto the
//! OrgStore primitives in `argo_identity::org`.

use crate::cli::cli_org_cmds::OrgCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{
    IngestSummary, MemberKind, OrgChartYaml, OrgMember, OrgStore, OrgUnit,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Top-level `argo org` dispatch.
pub async fn run(cmd: OrgCmd) -> ArgoResult<()> {
    let org = open_org_store().await?;
    org.ensure_schema().await?;
    match cmd {
        OrgCmd::Tree { tenant, json } => tree(&org, &tenant, json).await,
        OrgCmd::Member { tenant, name_or_alias, json } => {
            member(&org, &tenant, &name_or_alias, json).await
        }
        OrgCmd::ReportsTo { tenant, member, json } => {
            reports_to(&org, &tenant, &member, json).await
        }
        OrgCmd::Ingest { path, json } => ingest(&org, &path, json).await,
    }
}

async fn tree(org: &OrgStore, tenant: &str, as_json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let units = org.list_units(tenant_id).await?;
    let members = org.list_members(tenant_id).await?;
    if as_json {
        println!(
            "{}",
            json!({"units": units, "members": members})
        );
    } else {
        render_ascii_tree(&units, &members);
    }
    Ok(())
}

async fn member(
    org: &OrgStore,
    tenant: &str,
    name_or_alias: &str,
    as_json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let resolved = org.resolve(tenant_id, name_or_alias).await?;
    match resolved {
        Some(m) => {
            if as_json {
                println!("{}", serde_json::to_string(&m).unwrap_or_default());
            } else {
                print_member(&m);
            }
            Ok(())
        }
        None => Err(ArgoError::Provider(format!(
            "no org member matches '{name_or_alias}' in tenant '{tenant}'"
        ))),
    }
}

async fn reports_to(
    org: &OrgStore,
    tenant: &str,
    name_or_alias: &str,
    as_json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let m = org
        .resolve(tenant_id, name_or_alias)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "reports-to: no org member matches '{name_or_alias}'"
            ))
        })?;
    let chain = org.reports_to_chain(tenant_id, &m.id).await?;
    if as_json {
        println!(
            "{}",
            json!({"member": m, "chain": chain})
        );
    } else {
        println!("reports-to chain for '{}':", m.canonical);
        if chain.is_empty() {
            println!("  (top of org)");
        } else {
            for (i, step) in chain.iter().enumerate() {
                println!("  {}. {}", i + 1, step.canonical);
            }
        }
    }
    Ok(())
}

async fn ingest(org: &OrgStore, path: &std::path::Path, as_json: bool) -> ArgoResult<()> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let chart: OrgChartYaml = serde_yaml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let tenant_id = parse_tenant(&chart.tenant)?;
    let summary = org.ingest(tenant_id, &chart).await?;
    if as_json {
        println!("{}", serde_json::to_string(&summary).unwrap_or_default());
    } else {
        print_ingest(&summary);
    }
    Ok(())
}

fn render_ascii_tree(units: &[OrgUnit], members: &[OrgMember]) {
    if units.is_empty() && members.is_empty() {
        println!("(no org chart for tenant)");
        return;
    }
    let by_id: BTreeMap<&str, &OrgUnit> = units.iter().map(|u| (u.id.as_str(), u)).collect();
    let mut children: BTreeMap<Option<&str>, Vec<&OrgUnit>> = BTreeMap::new();
    for u in units {
        children
            .entry(u.parent_id.as_deref())
            .or_default()
            .push(u);
    }
    let mut members_by_unit: BTreeMap<Option<&str>, Vec<&OrgMember>> = BTreeMap::new();
    for m in members {
        members_by_unit
            .entry(m.unit_id.as_deref())
            .or_default()
            .push(m);
    }
    println!("org chart:");
    for root in children.get(&None).cloned().unwrap_or_default() {
        render_unit(root, &children, &members_by_unit, &by_id, 0);
    }
    let orphans = members_by_unit.get(&None).cloned().unwrap_or_default();
    if !orphans.is_empty() {
        println!("(no unit)");
        for m in orphans {
            println!("  - {}", m.canonical);
        }
    }
}

fn render_unit(
    unit: &OrgUnit,
    children: &BTreeMap<Option<&str>, Vec<&OrgUnit>>,
    members_by_unit: &BTreeMap<Option<&str>, Vec<&OrgMember>>,
    _by_id: &BTreeMap<&str, &OrgUnit>,
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    println!("{indent}{}", unit.name);
    if let Some(ms) = members_by_unit.get(&Some(unit.id.as_str())) {
        for m in ms {
            let kind = match m.kind {
                MemberKind::Human => "human",
                MemberKind::Worker => "worker",
            };
            println!("{indent}  - {} ({kind})", m.canonical);
        }
    }
    if let Some(subs) = children.get(&Some(unit.id.as_str())) {
        for sub in subs {
            render_unit(sub, children, members_by_unit, _by_id, depth + 1);
        }
    }
}

fn print_member(m: &OrgMember) {
    let kind = match m.kind {
        MemberKind::Human => "human",
        MemberKind::Worker => "worker",
    };
    println!("canonical:    {}", m.canonical);
    println!("kind:         {kind}");
    println!("id:           {}", m.id);
    if !m.aliases.is_empty() {
        println!("aliases:      {}", m.aliases.join(", "));
    }
    if let Some(unit) = m.unit_id.as_deref() {
        println!("unit:         {unit}");
    }
    if let Some(manager) = m.manager_id.as_deref() {
        println!("manager:      {manager}");
    }
    if let Some(worker) = m.worker_id.as_deref() {
        println!("worker_id:    {worker}");
    }
}

fn print_ingest(summary: &IngestSummary) {
    println!(
        "ingest ok: {} units, {} members, {} aliases, {} reports-to links",
        summary.units, summary.members, summary.aliases, summary.report_links
    );
}

fn parse_tenant(s: &str) -> ArgoResult<TenantId> {
    s.parse::<uuid::Uuid>()
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{s}': {e}")))
}

async fn open_org_store() -> ArgoResult<OrgStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(OrgStore::new(Arc::new(store)))
}
