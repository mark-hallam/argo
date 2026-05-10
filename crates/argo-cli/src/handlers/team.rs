//! `argo team` handler — operator-facing team primitive.
//! Stream `claude-worker-team-roster-cli` (V1.1).
//!
//! Bridges the org-chart (`org_unit`), roster (`roster`), and
//! roster-membership (`roster_member`) tables behind a single
//! `team` noun. Charter summaries for each member are stitched in
//! via [`argo_identity::IdentityStore`] so the column-existence
//! contract for `worker.charter_md` stays owned by that crate.
//!
//! All commands honor the `--tenant <uuid>` flag. The brief did
//! not call out a tenant flag explicitly; the surrounding `argo
//! org` and `argo roster` CLIs both require it, so requiring it
//! here is the documented sensible deviation.
//!
//! Mutating subcommands (add / remove / rename / create) live in
//! the sibling [`super::team_mutations`] module so this file stays
//! under the 500-LOC ceiling.

use crate::cli::cli_team_cmds::TeamCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{IdentityStore, WorkerId};
use argo_store::team_roster::{
    self, summarise_charter, TeamDetail, TeamMemberRow, TeamRow,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use std::str::FromStr;
use uuid::Uuid;

use super::team_mutations;

/// Top-level dispatch for `argo team ...`.
pub async fn run(cmd: TeamCmd) -> ArgoResult<()> {
    match cmd {
        TeamCmd::List { tenant, format } => list(&tenant, &format).await,
        TeamCmd::Show {
            tenant,
            team_id,
            format,
        } => show(&tenant, &team_id, &format).await,
        TeamCmd::Add {
            tenant,
            team_id,
            worker_id,
            role,
            json,
        } => team_mutations::add(&tenant, &team_id, &worker_id, role.as_deref(), json).await,
        TeamCmd::Remove {
            tenant,
            team_id,
            worker_id,
            json,
        } => team_mutations::remove(&tenant, &team_id, &worker_id, json).await,
        TeamCmd::Rename {
            tenant,
            team_id,
            new_name,
            json,
        } => team_mutations::rename(&tenant, &team_id, &new_name, json).await,
        TeamCmd::Create {
            tenant,
            name,
            parent,
            json,
        } => team_mutations::create(&tenant, &name, parent.as_deref(), json).await,
    }
}

#[derive(Debug, Serialize)]
struct ListPayload<'a> {
    teams: &'a [TeamRow],
}

async fn list(tenant: &str, format: &str) -> ArgoResult<()> {
    let fmt = OutputFormat::parse(format)?;
    let store = open_store().await?;
    store.ensure_team_roster_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let teams = team_roster::list_teams(store.pool(), tenant_id).await?;
    match fmt {
        OutputFormat::Json => print_json(&ListPayload { teams: &teams })?,
        OutputFormat::Markdown => print_list_markdown(&teams),
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub(super) struct ShowMember {
    pub(super) worker_id: String,
    pub(super) role: Option<String>,
    pub(super) added_at: i64,
    pub(super) charter_summary: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ShowPayload {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) parent_id: Option<String>,
    pub(super) level: u32,
    pub(super) members: Vec<ShowMember>,
}

async fn show(tenant: &str, team_id: &str, format: &str) -> ArgoResult<()> {
    let fmt = OutputFormat::parse(format)?;
    let store = open_store().await?;
    store.ensure_team_roster_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let detail = team_roster::get_team(store.pool(), tenant_id, team_id)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "no team '{team_id}' in tenant '{tenant}'"
            ))
        })?;
    let payload = hydrate_show(detail, tenant_id, &store).await?;
    match fmt {
        OutputFormat::Json => print_json(&payload)?,
        OutputFormat::Markdown => print_show_markdown(&payload),
    }
    Ok(())
}

async fn hydrate_show(
    detail: TeamDetail,
    tenant: TenantId,
    store: &SessionStore,
) -> ArgoResult<ShowPayload> {
    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;
    let mut members = Vec::with_capacity(detail.members.len());
    for m in detail.members {
        let summary = lookup_charter_summary(&identities, tenant, &m).await?;
        members.push(ShowMember {
            worker_id: m.worker_id,
            role: m.role,
            added_at: m.added_at,
            charter_summary: summary,
        });
    }
    Ok(ShowPayload {
        id: detail.id,
        name: detail.name,
        parent_id: detail.parent_id,
        level: detail.level,
        members,
    })
}

async fn lookup_charter_summary(
    identities: &IdentityStore<'_>,
    tenant: TenantId,
    member: &TeamMemberRow,
) -> ArgoResult<Option<String>> {
    let Ok(uuid) = Uuid::parse_str(&member.worker_id) else {
        return Ok(None);
    };
    let worker_id = WorkerId(uuid);
    match identities.get_worker(tenant, worker_id).await? {
        Some(worker) => Ok(worker.charter.as_deref().map(summarise_charter)),
        None => Ok(None),
    }
}

fn print_list_markdown(teams: &[TeamRow]) {
    print!("{}", render_list_markdown(teams));
}

fn print_show_markdown(payload: &ShowPayload) {
    print!("{}", render_show_markdown(payload));
}

fn render_list_markdown(teams: &[TeamRow]) -> String {
    if teams.is_empty() {
        return "(no teams)\n".to_string();
    }
    let mut out = String::from(
        "# Teams\n\n| name | id | parent | members |\n|------|----|--------|---------|\n",
    );
    for t in teams {
        let parent = t.parent_id.as_deref().unwrap_or("—");
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            escape_md(&t.name),
            t.id,
            parent,
            t.member_count
        ));
    }
    out
}

fn render_show_markdown(payload: &ShowPayload) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Team `{}`\n\n", escape_md(&payload.name)));
    out.push_str(&format!("- id: `{}`\n", payload.id));
    match payload.parent_id.as_deref() {
        Some(p) => out.push_str(&format!("- parent: `{p}`\n")),
        None => out.push_str("- parent: (root)\n"),
    }
    out.push_str(&format!("- level: {}\n", payload.level));
    out.push_str(&format!("- members: {}\n\n", payload.members.len()));
    if payload.members.is_empty() {
        out.push_str("_no active members._\n");
        return out;
    }
    out.push_str("## Members\n\n");
    out.push_str("| worker_id | role | level | charter |\n");
    out.push_str("|-----------|------|-------|---------|\n");
    for m in &payload.members {
        let role = m.role.as_deref().unwrap_or("—");
        let charter = m
            .charter_summary
            .as_deref()
            .map(escape_md)
            .unwrap_or_else(|| "—".to_string());
        out.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            m.worker_id,
            escape_md(role),
            payload.level,
            charter
        ));
    }
    out
}

fn escape_md(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

pub(super) fn print_json<T: Serialize>(payload: &T) -> ArgoResult<()> {
    let s = serde_json::to_string_pretty(payload)
        .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
    println!("{s}");
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Markdown,
    Json,
}

impl OutputFormat {
    fn parse(value: &str) -> ArgoResult<Self> {
        match value {
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            other => Err(ArgoError::Provider(format!(
                "invalid --format '{other}': expected markdown|json"
            ))),
        }
    }
}

pub(super) async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

pub(super) fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::from_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}

#[cfg(test)]
pub(super) async fn __test_show_payload(
    detail: TeamDetail,
    tenant: TenantId,
    store: &SessionStore,
) -> ArgoResult<ShowPayload> {
    hydrate_show(detail, tenant, store).await
}

#[cfg(test)]
pub(super) fn __test_render_list(teams: &[TeamRow]) -> String {
    render_list_markdown(teams)
}

#[cfg(test)]
pub(super) fn __test_render_show(payload: &ShowPayload) -> String {
    render_show_markdown(payload)
}

#[cfg(test)]
#[path = "team_tests.rs"]
mod tests;
