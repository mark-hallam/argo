//! Per-tenant MCP connector pool: spawns connector subprocesses through
//! the warm [`McpClientPool`] and injects the active tenant's stored
//! credentials at spawn (the connector-reach unlock).
use argo_mcp::{McpClient, McpClientPool, PoolConfig};
use argo_types::TenantId;
use serde_json::Value;
use std::sync::Arc;

use super::parse_stdio_transport;

/// Wrapper around the process-wide [`McpClientPool`] (shipped in
/// `mission-mcp-connection-pooling`). The agent-runner used to spawn a
/// fresh subprocess per turn via a per-turn `HashMap`; the wrapper
/// preserves the existing `ToolRegistry::set_mcp_pool` injection seam
/// while routing every acquisition through the warm pool. Subprocesses
/// stay live across turns; LRU + idle-TTL eviction reaps stale entries.
pub(crate) struct McpConnectionPool {
    pool: Arc<McpClientPool>,
    /// Workspace root used to load the active tenant's stored credential
    /// section at connector spawn (the `<workspace>/.argo/credentials.toml`
    /// per-tenant TOML the gateway writes). `None` in legacy/test contexts
    /// that never inject creds — those spawn a bare connector exactly as
    /// before.
    workspace: Option<std::path::PathBuf>,
}

impl Default for McpConnectionPool {
    fn default() -> Self {
        Self {
            pool: McpClientPool::new(PoolConfig::default()),
            workspace: None,
        }
    }
}

impl McpConnectionPool {
    /// Build a pool that injects per-tenant credentials at connector spawn,
    /// loading them from `workspace`'s credential vault.
    pub(crate) fn with_workspace(workspace: std::path::PathBuf) -> Self {
        Self {
            pool: McpClientPool::new(PoolConfig::default()),
            workspace: Some(workspace),
        }
    }
}

impl std::fmt::Debug for McpConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpConnectionPool")
            .field("pool", &self.pool)
            .field("workspace", &self.workspace)
            .finish()
    }
}

impl McpConnectionPool {
    pub(crate) async fn client(
        &self,
        tenant_id: TenantId,
        name: &str,
        transport: &Value,
    ) -> Result<Arc<McpClient>, String> {
        let (command, args) = parse_stdio_transport(transport)?;
        // SECURITY-CRITICAL (per-tenant credential injection). The env we
        // build below is derived ONLY from `tenant_id` — `connector_env`
        // calls `argo_config::cred::load_tenant(workspace, &tenant_id)`,
        // which reads exclusively the `[credentials.tenant.<tenant_id>]`
        // section and decrypts it under that tenant's own master key. No
        // other tenant's section is ever read, so the connector process for
        // tenant A can NEVER receive tenant B's credentials. The pool also
        // keys cached subprocesses by `(tenant_id, manifest_id)`, so a warm
        // connector spawned for one tenant is never handed to another.
        let env = self.connector_env(tenant_id);
        let guard = self
            .pool
            .acquire(tenant_id, name.to_string(), || async move {
                McpClient::connect_stdio_with_env(
                    &command,
                    &args,
                    &env,
                    argo_mcp::DEFAULT_CALL_TIMEOUT,
                )
                .await
            })
            .await
            .map_err(|e| format!("connect MCP server '{name}': {e}"))?;
        Ok(guard.client_arc())
    }

    /// Build the env var list injected into a connector subprocess for
    /// `tenant_id`: every stored credential in THAT tenant's vault section,
    /// plus `ARGO_MCP_TENANT_ID` so the connector tags its own per-tenant
    /// state (e.g. OAuth token cache) under the right tenant.
    ///
    /// Strictly tenant-scoped: the only credentials returned come from
    /// `load_tenant`, which reads exclusively the calling tenant's section.
    fn connector_env(&self, tenant_id: TenantId) -> Vec<(String, String)> {
        let mut env = vec![("ARGO_MCP_TENANT_ID".to_string(), tenant_id.to_string())];
        if let Some(workspace) = &self.workspace {
            if let Ok(creds) = argo_config::cred::load_tenant(workspace, &tenant_id) {
                env.extend(creds);
            }
        }
        env
    }
}

#[cfg(test)]
#[path = "dispatch_mcp_pool_tests.rs"]
mod dispatch_mcp_pool_tests;
