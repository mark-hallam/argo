//! Per-tenant MCP server registry query helpers.
//!
//! Stream `codex-mcp-server-registry-cli`. Schema applied lazily via
//! [`McpRegistryStore::ensure_schema`] from the sibling raw SQL file
//! `migrations/33__mcp_registry.sql` — same pattern as
//! `worker_archive` / `handoff_resolved_at`. Production callers
//! reach this through the `argo mcp registry list/add/remove` CLI;
//! the dispatch layer also reads `get(tenant, server_id)` when the
//! `ARGO_MCP_REGISTRY_BACKED=1` feature flag is set.

use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

const SCHEMA_SQL: &str = include_str!("../../migrations/33__mcp_registry.sql");

/// One registered MCP server row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpServerRow {
    /// Tenant owning this registration.
    pub tenant_id: String,
    /// Operator-chosen server id referenced by actions.
    pub server_id: String,
    /// Server kind or transport family.
    pub kind: String,
    /// Base URL for remote MCP servers.
    pub base_url: String,
    /// Opaque reference into the existing credential store.
    pub secrets_ref: Option<String>,
    /// Unix timestamp seconds when the row was registered.
    pub registered_at: i64,
    /// Unix timestamp seconds for last successful best-effort ping.
    pub last_seen: Option<i64>,
    /// Last ping/health error, if any.
    pub last_error: Option<String>,
}

/// Insert payload for [`McpRegistryStore::add`].
#[derive(Debug, Clone, Copy)]
pub struct NewMcpServer<'a> {
    /// Tenant owning this registration.
    pub tenant_id: &'a str,
    /// Operator-chosen server id referenced by actions.
    pub server_id: &'a str,
    /// Server kind or transport family.
    pub kind: &'a str,
    /// Base URL for remote MCP servers.
    pub base_url: &'a str,
    /// Opaque reference into the existing credential store.
    pub secrets_ref: Option<&'a str>,
    /// Unix timestamp seconds for last successful ping.
    pub last_seen: Option<i64>,
    /// Last ping/health error, if any.
    pub last_error: Option<&'a str>,
}

/// Thin wrapper around a SQLite pool for MCP registry operations.
///
/// ```no_run
/// use argo_store::queries::mcp_registry::McpRegistryStore;
/// use argo_store::SessionStore;
/// # async fn run() -> Result<(), argo_types::ArgoError> {
/// let store = SessionStore::open(":memory:").await?;
/// store.migrate().await?;
/// let registry = McpRegistryStore::new(store.pool());
/// let _rows = registry.list("tenant-a").await?;
/// # Ok(()) }
/// ```
pub struct McpRegistryStore<'a> {
    pool: &'a SqlitePool,
}

impl<'a> McpRegistryStore<'a> {
    /// Bind registry operations to a SQLite pool.
    #[must_use]
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Apply the registry schema idempotently. Strips `--` line
    /// comments before splitting on `;` so a stray semicolon inside
    /// a header comment can't blow up the naive splitter.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        let stripped: String = SCHEMA_SQL
            .lines()
            .map(|line| match line.find("--") {
                Some(idx) => &line[..idx],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n");
        for stmt in stripped.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("mcp registry schema: {e}")))?;
        }
        Ok(())
    }

    /// Insert one server registration.
    pub async fn add(&self, new: NewMcpServer<'_>, now: DateTime<Utc>) -> ArgoResult<McpServerRow> {
        self.ensure_schema().await?;
        if self.get(new.tenant_id, new.server_id).await?.is_some() {
            return Err(ArgoError::Provider(format!(
                "mcp registry conflict: server_id '{}' already exists for tenant '{}'",
                new.server_id, new.tenant_id
            )));
        }
        sqlx::query(
            "INSERT INTO mcp_server_registry \
             (tenant_id, server_id, kind, base_url, secrets_ref, registered_at, last_seen, last_error) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(new.tenant_id)
        .bind(new.server_id)
        .bind(new.kind)
        .bind(new.base_url)
        .bind(new.secrets_ref)
        .bind(now.timestamp())
        .bind(new.last_seen)
        .bind(new.last_error)
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("mcp registry add: {e}")))?;
        self.get(new.tenant_id, new.server_id)
            .await?
            .ok_or_else(|| ArgoError::Provider("mcp registry add: inserted row disappeared".into()))
    }

    /// List rows for one tenant, newest successful ping first.
    pub async fn list(&self, tenant_id: &str) -> ArgoResult<Vec<McpServerRow>> {
        self.ensure_schema().await?;
        let rows: Vec<RowTuple> = sqlx::query_as(SELECT_BY_TENANT)
            .bind(tenant_id)
            .fetch_all(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("mcp registry list: {e}")))?;
        Ok(rows.into_iter().map(into_row).collect())
    }

    /// Fetch one row by tenant + server id.
    pub async fn get(&self, tenant_id: &str, server_id: &str) -> ArgoResult<Option<McpServerRow>> {
        self.ensure_schema().await?;
        let row: Option<RowTuple> = sqlx::query_as(SELECT_ONE)
            .bind(tenant_id)
            .bind(server_id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("mcp registry get: {e}")))?;
        Ok(row.map(into_row))
    }

    /// Remove one row. Returns the deleted row, or NotFound surfaced
    /// as a `Provider` error (no `ArgoError::NotFound` variant).
    pub async fn remove(&self, tenant_id: &str, server_id: &str) -> ArgoResult<McpServerRow> {
        self.ensure_schema().await?;
        let row = self.get(tenant_id, server_id).await?.ok_or_else(|| {
            ArgoError::Provider(format!(
                "mcp registry not found: server_id '{server_id}' for tenant '{tenant_id}'"
            ))
        })?;
        sqlx::query("DELETE FROM mcp_server_registry WHERE tenant_id = ? AND server_id = ?")
            .bind(tenant_id)
            .bind(server_id)
            .execute(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("mcp registry remove: {e}")))?;
        Ok(row)
    }
}

type RowTuple = (
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
    Option<i64>,
    Option<String>,
);

const SELECT_BY_TENANT: &str = "SELECT tenant_id, server_id, kind, base_url, secrets_ref, \
                                registered_at, last_seen, last_error \
                                FROM mcp_server_registry \
                                WHERE tenant_id = ? \
                                ORDER BY COALESCE(last_seen, registered_at) DESC, server_id ASC";
const SELECT_ONE: &str = "SELECT tenant_id, server_id, kind, base_url, secrets_ref, \
                          registered_at, last_seen, last_error \
                          FROM mcp_server_registry WHERE tenant_id = ? AND server_id = ?";

fn into_row(row: RowTuple) -> McpServerRow {
    McpServerRow {
        tenant_id: row.0,
        server_id: row.1,
        kind: row.2,
        base_url: row.3,
        secrets_ref: row.4,
        registered_at: row.5,
        last_seen: row.6,
        last_error: row.7,
    }
}
