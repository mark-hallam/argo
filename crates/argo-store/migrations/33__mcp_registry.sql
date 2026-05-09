-- Stream `codex-mcp-server-registry-cli`. Per-tenant MCP server
-- registry, replaces hand-edits to `manifests/connectors/mcp.toml`
-- for onboarding tenant-specific servers. The dispatch path in
-- `worker_actions_executor_mcp` reads from this table when
-- `ARGO_MCP_REGISTRY_BACKED=1`, otherwise it falls back to the
-- vertical manifest's `[mcp_servers]` map. `secrets_ref` is an
-- opaque pointer into the existing credential store, so raw
-- secrets never land here.
CREATE TABLE IF NOT EXISTS mcp_server_registry (
  tenant_id     TEXT NOT NULL,
  server_id     TEXT NOT NULL,
  kind          TEXT NOT NULL,
  base_url      TEXT NOT NULL,
  secrets_ref   TEXT,
  registered_at INTEGER NOT NULL,
  last_seen     INTEGER,
  last_error    TEXT,
  PRIMARY KEY (tenant_id, server_id)
);

CREATE INDEX IF NOT EXISTS idx_mcp_server_registry_tenant_last_seen
  ON mcp_server_registry (tenant_id, last_seen);
