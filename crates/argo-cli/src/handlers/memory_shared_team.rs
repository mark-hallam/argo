//! `argo memory shared-team` commands.

use crate::cli::SharedTeamCmd;
use argo_identity::WorkerId;
use argo_memory::{shared_team, MemoryStore, SharedEntry};
use argo_provider::{resolve_latest, DiscoverySpec, PreferRule};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct SharedTeamList {
    tenant_id: String,
    count: usize,
    entries: Vec<SharedTeamRow>,
}

#[derive(Debug, Serialize)]
struct SharedTeamGrep {
    tenant_id: String,
    pattern: String,
    count: usize,
    entries: Vec<SharedTeamRow>,
}

#[derive(Debug, Serialize)]
struct SharedTeamWrite {
    tenant_id: String,
    entry: SharedTeamRow,
}

#[derive(Debug, Serialize)]
struct SharedTeamRow {
    id: String,
    author_worker_id: String,
    content: String,
    written_at: String,
}

pub async fn run(memory: &MemoryStore, cmd: SharedTeamCmd) -> ArgoResult<()> {
    match cmd {
        SharedTeamCmd::List {
            tenant,
            limit,
            json,
        } => list(memory, &tenant, limit, json).await,
        SharedTeamCmd::Write {
            tenant,
            author,
            content,
            json,
        } => write(memory, &tenant, &author, &content, json).await,
        SharedTeamCmd::Grep {
            tenant,
            pattern,
            json,
        } => grep(memory, &tenant, &pattern, json).await,
    }
}

async fn list(
    memory: &MemoryStore,
    tenant: &str,
    limit: Option<u32>,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let mut entries = read_newest_first(memory, tenant_id).await?;
    if let Some(limit) = limit {
        entries.truncate(limit as usize);
    }
    let rows = rows(entries);
    if json {
        print_json(&SharedTeamList {
            tenant_id: tenant_id.to_string(),
            count: rows.len(),
            entries: rows,
        })
    } else {
        print_rows(&rows);
        Ok(())
    }
}

async fn write(
    memory: &MemoryStore,
    tenant: &str,
    author: &str,
    content: &str,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let author = parse_worker(author)?;
    let entry = memory
        .write_shared_team(&shared_team(tenant_id), author, content.to_string())
        .await?;
    let row = row(entry);
    if json {
        print_json(&SharedTeamWrite {
            tenant_id: tenant_id.to_string(),
            entry: row,
        })
    } else {
        println!(
            "wrote shared-team entry {} for tenant {} by {}",
            row.id, tenant_id, row.author_worker_id
        );
        Ok(())
    }
}

async fn grep(memory: &MemoryStore, tenant: &str, pattern: &str, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let spec = compile_case_insensitive(pattern)?;
    let entries = read_newest_first(memory, tenant_id)
        .await?
        .into_iter()
        .filter(|entry| pattern_matches(&spec, &entry.content))
        .collect::<Vec<_>>();
    let rows = rows(entries);
    if json {
        print_json(&SharedTeamGrep {
            tenant_id: tenant_id.to_string(),
            pattern: pattern.to_string(),
            count: rows.len(),
            entries: rows,
        })
    } else {
        print_rows(&rows);
        Ok(())
    }
}

async fn read_newest_first(
    memory: &MemoryStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<SharedEntry>> {
    let mut entries = memory.read_shared_team(&shared_team(tenant_id)).await?;
    entries.sort_by_key(|entry| std::cmp::Reverse((entry.written_at, entry.id)));
    Ok(entries)
}

fn rows(entries: Vec<SharedEntry>) -> Vec<SharedTeamRow> {
    entries.into_iter().map(row).collect()
}

fn row(entry: SharedEntry) -> SharedTeamRow {
    SharedTeamRow {
        id: entry.id.to_string(),
        author_worker_id: entry.author_worker_id.to_string(),
        content: entry.content,
        written_at: entry.written_at.to_rfc3339(),
    }
}

fn print_rows(rows: &[SharedTeamRow]) {
    if rows.is_empty() {
        println!("(no shared-team memory entries)");
        return;
    }
    println!(
        "{:<36} {:<36} {:<25} content",
        "id", "author_worker_id", "written_at"
    );
    for row in rows {
        println!(
            "{:<36} {:<36} {:<25} {}",
            row.id,
            row.author_worker_id,
            row.written_at,
            one_line(&row.content)
        );
    }
}

fn one_line(content: &str) -> String {
    content.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn compile_case_insensitive(pattern: &str) -> ArgoResult<DiscoverySpec> {
    let spec = DiscoverySpec {
        endpoint: String::new(),
        include_pattern: format!("(?i:{pattern})"),
        exclude_pattern: None,
        prefer: PreferRule::LexicographicMax,
        refresh_interval_hours: 24,
    };
    let empty: [&str; 0] = [];
    match resolve_latest(&spec, &empty) {
        Err(ArgoError::Provider(message)) if message.starts_with("invalid include_pattern") => Err(
            ArgoError::Provider(format!("400 invalid regex '{pattern}': {message}")),
        ),
        _ => Ok(spec),
    }
}

fn pattern_matches(spec: &DiscoverySpec, content: &str) -> bool {
    resolve_latest(spec, &[content]).is_ok()
}

fn parse_tenant(raw: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(raw)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{raw}': {e}")))
}

fn parse_worker(raw: &str) -> ArgoResult<WorkerId> {
    raw.parse()
        .map_err(|e| ArgoError::Provider(format!("worker id '{raw}': {e}")))
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialize shared-team memory: {e}")))?;
    println!("{body}");
    Ok(())
}
