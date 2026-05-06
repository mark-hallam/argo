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
- `crates/argo-mcp/src/connectors/jira.rs` (NEW, ~450 LOC
  cap)
- `crates/argo-mcp/src/connectors/mod.rs` — narrowly: add
  `jira` module
- `crates/argo-cli/src/handlers/connector_jira.rs` (NEW)
- `crates/argo-cli/src/cli_connector_cmds.rs` — narrowly:
  `Jira` subcommand variant
- BLANKET CLI ALLOWLIST applies

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
