//! `[runtime.mcp_pool]` — operator-tunable knobs for the warm MCP
//! subprocess pool (`mission-mcp-connection-pooling`,
//! MISSION_BACKLOG §8 #252).
//!
//! Sibling of [`crate::sections_context`]; the parent module re-exports
//! these symbols so external imports stay `argo_config::McpPoolConfig`.
use serde::{Deserialize, Serialize};

/// Default LRU capacity of the MCP-client connection pool. Mirrors
/// `argo_mcp::DEFAULT_POOL_MAX_CLIENTS`.
pub const DEFAULT_MCP_POOL_MAX_CLIENTS: u32 = 32;
/// Default idle-TTL for the MCP-client pool, in seconds. Mirrors
/// `argo_mcp::DEFAULT_POOL_IDLE_TTL_SECS`.
pub const DEFAULT_MCP_POOL_IDLE_TTL_SECS: u64 = 600;

/// `[runtime.mcp_pool]` — operator-tunable knobs for the warm MCP
/// subprocess pool (`mission-mcp-connection-pooling`,
/// MISSION_BACKLOG §8 #252). The pool itself lives in
/// `argo_mcp::pool`; this struct mirrors the TOML-side knobs and is
/// mapped at AgentDeps wiring time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct McpPoolConfig {
    /// Hard cap on cached MCP clients per pool. New acquisitions
    /// past this evict the LRU entry.
    pub max_clients: u32,
    /// Seconds an entry can sit untouched before the idle reaper
    /// removes it on the next tick.
    pub idle_ttl_secs: u64,
}

impl Default for McpPoolConfig {
    fn default() -> Self {
        Self {
            max_clients: DEFAULT_MCP_POOL_MAX_CLIENTS,
            idle_ttl_secs: DEFAULT_MCP_POOL_IDLE_TTL_SECS,
        }
    }
}
