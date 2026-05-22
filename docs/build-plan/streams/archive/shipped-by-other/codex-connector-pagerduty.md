# V1.2 — PagerDuty MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

PagerDuty connector. Auth via API token (`PAGERDUTY_API_KEY`).

- Methods: `list_incidents`, `get_incident`, `create_incident`,
  `acknowledge_incident`, `resolve_incident`, `list_services`,
  `list_oncalls`
- Read autonomous-safe; create/ack/resolve approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-pagerduty/` mirroring
`argo-mcp-connector-asana`:
- `crates/argo-mcp-connector-pagerduty/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-pagerduty/src/{lib,main,auth,client,schemas,service}.rs` (NEW)
- `Cargo.toml` workspace — narrowly: add new member

NO CLI subcommand.

## Acceptance criteria
- [ ] All 7 methods callable
- [ ] Token bearer auth
- [ ] Approval-gate marks correct
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-pagerduty`
- PR title: `mcp: native PagerDuty connector (codex-connector-pagerduty)`
