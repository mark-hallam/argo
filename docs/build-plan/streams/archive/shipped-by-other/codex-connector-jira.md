# V1.2 — Jira Cloud MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Jira Cloud connector. Auth via email + API token + base URL
(`JIRA_EMAIL`, `JIRA_API_TOKEN`, `JIRA_BASE_URL`).

- Methods: `list_issues` (JQL search), `get_issue`,
  `create_issue`, `update_issue`, `comment_issue`,
  `transition_issue`, `list_projects`
- Read methods autonomous-safe; create/update/comment/
  transition routed through approval

## Files you may touch

NEW crate at `crates/argo-mcp-connector-jira/` mirroring the
file layout of `argo-mcp-connector-slack`:
- `crates/argo-mcp-connector-jira/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-jira/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-jira/src/main.rs` (NEW)
- `crates/argo-mcp-connector-jira/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-jira/src/client.rs` (NEW)
- `crates/argo-mcp-connector-jira/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-jira/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

## Acceptance criteria
- [ ] All 7 methods callable
- [ ] JQL passthrough on `list_issues`
- [ ] Approval-gate marks set correctly per method
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-jira`
- PR title: `mcp: native Jira Cloud connector (codex-connector-jira)`
