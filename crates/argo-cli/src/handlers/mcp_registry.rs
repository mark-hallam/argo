//! `argo mcp registry` — per-tenant MCP server registrations.
//!
//! Stream `codex-mcp-server-registry-cli`. Surfaces the registry
//! table managed by `argo_store::queries::mcp_registry` through three
//! subcommands: `list`, `add`, `remove`. The dispatch path in
//! [`super::worker_actions_executor_mcp`] reads the same table when
//! `ARGO_MCP_REGISTRY_BACKED=1` is set, so adds here surface to
//! action execution without a redeploy.

use crate::cli::McpRegistryCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::queries::mcp_registry::{McpRegistryStore, McpServerRow, NewMcpServer};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

/// Dispatch `argo mcp registry ...` commands.
pub async fn run(cmd: McpRegistryCmd) -> ArgoResult<()> {
    match cmd {
        McpRegistryCmd::List {
            tenant,
            format,
            json,
        } => cmd_list(tenant.as_deref(), &format, json).await,
        McpRegistryCmd::Add {
            server_id,
            kind,
            base_url,
            secrets_ref,
            tenant,
            json,
        } => {
            cmd_add(
                tenant.as_deref(),
                &server_id,
                &kind,
                &base_url,
                secrets_ref.as_deref(),
                json,
            )
            .await
        }
        McpRegistryCmd::Remove {
            server_id,
            tenant,
            force,
            json,
        } => cmd_remove(tenant.as_deref(), &server_id, force, json).await,
    }
}

/// List MCP registry rows for one tenant.
pub async fn cmd_list(tenant: Option<&str>, format: &str, json_out: bool) -> ArgoResult<()> {
    let (store, tenant_id) = open_store_and_tenant(tenant).await?;
    let rows = McpRegistryStore::new(store.pool()).list(&tenant_id).await?;
    let pick = if json_out { "json" } else { format };
    match pick {
        "json" => {
            for row in rows.iter().map(DisplayRow::from) {
                println!(
                    "{}",
                    serde_json::to_string(&row)
                        .map_err(|e| ArgoError::Provider(format!("registry JSON: {e}")))?
                );
            }
            Ok(())
        }
        "markdown" => {
            print!("{}", render_markdown(&rows));
            Ok(())
        }
        other => Err(ArgoError::Provider(format!(
            "unknown registry format '{other}' (expected markdown or json)"
        ))),
    }
}

/// Add one MCP registry row after URL validation and a best-effort ping.
pub async fn cmd_add(
    tenant: Option<&str>,
    server_id: &str,
    kind: &str,
    base_url: &str,
    secrets_ref: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    validate_server_id(server_id)?;
    validate_kind(kind)?;
    validate_url(base_url)?;
    let (store, tenant_id) = open_store_and_tenant(tenant).await?;
    let ping = ping_server(base_url).await;
    let row = McpRegistryStore::new(store.pool())
        .add(
            NewMcpServer {
                tenant_id: &tenant_id,
                server_id,
                kind,
                base_url,
                secrets_ref,
                last_seen: ping.last_seen,
                last_error: ping.last_error.as_deref(),
            },
            Utc::now(),
        )
        .await?;
    if json_out {
        print_json(&json!({"ok": true, "mcp_server": DisplayRow::from(&row)}))
    } else {
        println!(
            "registered MCP server {} for tenant {} ({})",
            row.server_id,
            row.tenant_id,
            status(&row)
        );
        Ok(())
    }
}

/// Remove one MCP registry row, refusing active actions unless forced.
pub async fn cmd_remove(
    tenant: Option<&str>,
    server_id: &str,
    force: bool,
    json_out: bool,
) -> ArgoResult<()> {
    validate_server_id(server_id)?;
    let (store, tenant_id) = open_store_and_tenant(tenant).await?;
    let active = active_actions_for_server(server_id).await?;
    let row = remove_with_active_count(
        &McpRegistryStore::new(store.pool()),
        &tenant_id,
        server_id,
        force,
        active,
    )
    .await?;
    if json_out {
        print_json(&json!({"ok": true, "removed": DisplayRow::from(&row)}))
    } else {
        println!(
            "removed MCP server {} for tenant {}",
            row.server_id, row.tenant_id
        );
        Ok(())
    }
}

async fn remove_with_active_count(
    registry: &McpRegistryStore<'_>,
    tenant_id: &str,
    server_id: &str,
    force: bool,
    active: usize,
) -> ArgoResult<McpServerRow> {
    if active > 0 && !force {
        return Err(ArgoError::Provider(format!(
            "mcp registry remove refused: server_id '{server_id}' has {active} active action(s); pass --force to remove anyway"
        )));
    }
    registry.remove(tenant_id, server_id).await
}

async fn open_store_and_tenant(tenant: Option<&str>) -> ArgoResult<(SessionStore, String)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let tenant_id = match tenant {
        Some(id) => {
            validate_tenant(id)?;
            id.to_string()
        }
        None => {
            let name = cwd
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("workspace");
            let root = cwd.to_string_lossy();
            let (tenant, _workspace) = store.ensure_workspace(name, &root).await?;
            tenant.to_string()
        }
    };
    Ok((store, tenant_id))
}

fn validate_tenant(tenant: &str) -> ArgoResult<()> {
    if tenant.trim().is_empty() {
        Err(ArgoError::Provider("mcp registry: empty tenant id".into()))
    } else {
        Ok(())
    }
}

fn validate_server_id(server_id: &str) -> ArgoResult<()> {
    let ok = !server_id.trim().is_empty()
        && server_id
            .chars()
            .all(|c| c == '-' || c == '_' || c == '.' || c.is_ascii_alphanumeric());
    if ok {
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "mcp registry: invalid server_id '{server_id}'"
        )))
    }
}

fn validate_kind(kind: &str) -> ArgoResult<()> {
    let ok = !kind.trim().is_empty()
        && kind
            .chars()
            .all(|c| c == '-' || c == '_' || c.is_ascii_alphanumeric());
    if ok {
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "mcp registry: invalid kind '{kind}'"
        )))
    }
}

fn validate_url(base_url: &str) -> ArgoResult<()> {
    let parsed = reqwest::Url::parse(base_url)
        .map_err(|e| ArgoError::Provider(format!("mcp registry: invalid --url: {e}")))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        other => Err(ArgoError::Provider(format!(
            "mcp registry: invalid --url scheme '{other}' (expected http or https)"
        ))),
    }
}

struct PingResult {
    last_seen: Option<i64>,
    last_error: Option<String>,
}

async fn ping_server(base_url: &str) -> PingResult {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return PingResult {
                last_seen: None,
                last_error: Some(format!("ping client: {e}")),
            };
        }
    };
    match client.get(base_url).send().await {
        Ok(resp) if resp.status().is_success() => PingResult {
            last_seen: Some(Utc::now().timestamp()),
            last_error: None,
        },
        Ok(resp) => PingResult {
            last_seen: None,
            last_error: Some(format!("ping returned {}", resp.status())),
        },
        Err(e) => PingResult {
            last_seen: None,
            last_error: Some(format!("ping failed: {e}")),
        },
    }
}

async fn active_actions_for_server(server_id: &str) -> ArgoResult<usize> {
    let action_servers = mcp_action_servers()?;
    let rows = super::worker_actions_query::collect_latest_actions(None).await?;
    Ok(rows
        .iter()
        .filter(|row| action_servers.get(&row.action_type).map(String::as_str) == Some(server_id))
        .filter(|row| {
            super::worker_actions::ActionState::parse(&row.state)
                .map(|state| !state.is_terminal())
                .unwrap_or(true)
        })
        .count())
}

fn mcp_action_servers() -> ArgoResult<HashMap<String, String>> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let mut out = HashMap::new();
    for (_id, loaded) in registry.list() {
        let Some(actions) = &loaded.manifest.actions else {
            continue;
        };
        for (action_id, decl) in &actions.actions {
            if let Some(server) = decl.executor.mcp_server() {
                out.insert(action_id.clone(), server.to_string());
            }
        }
    }
    Ok(out)
}

fn render_markdown(rows: &[McpServerRow]) -> String {
    let mut out = String::new();
    out.push_str("| server_id | kind | base_url | status | last_seen |\n");
    out.push_str("|---|---|---|---|---:|\n");
    for row in rows {
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} |\n",
            esc(&row.server_id),
            esc(&row.kind),
            esc(&row.base_url),
            status(row),
            row.last_seen
                .map(|ts| ts.to_string())
                .unwrap_or_else(|| "-".into())
        ));
    }
    out
}

fn esc(value: &str) -> String {
    value.replace('|', "\\|")
}

fn status(row: &McpServerRow) -> &'static str {
    if row.last_error.is_some() {
        "error"
    } else if row.last_seen.is_some() {
        "ok"
    } else {
        "unknown"
    }
}

#[derive(Serialize)]
struct DisplayRow<'a> {
    tenant_id: &'a str,
    server_id: &'a str,
    kind: &'a str,
    base_url: &'a str,
    status: &'static str,
    last_seen: Option<i64>,
    last_error: Option<&'a str>,
    secrets_ref: Option<&'a str>,
    registered_at: i64,
}

impl<'a> From<&'a McpServerRow> for DisplayRow<'a> {
    fn from(row: &'a McpServerRow) -> Self {
        Self {
            tenant_id: &row.tenant_id,
            server_id: &row.server_id,
            kind: &row.kind,
            base_url: &row.base_url,
            status: status(row),
            last_seen: row.last_seen,
            last_error: row.last_error.as_deref(),
            secrets_ref: row.secrets_ref.as_deref(),
            registered_at: row.registered_at,
        }
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|e| ArgoError::Provider(format!("registry JSON: {e}")))?
    );
    Ok(())
}

#[cfg(test)]
#[path = "mcp_registry_tests.rs"]
mod tests;
