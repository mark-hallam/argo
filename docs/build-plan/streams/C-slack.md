# Stream C — Slack MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

MCP server exposing Slack operations: `slack.post_message`,
`slack.list_channels`, `slack.read_recent`, `slack.dm_user`,
`slack.search`. Uses Slack Web API with bot token.

## Files you may touch
- `crates/argo-mcp-connector-slack/**` (NEW crate)
- `Cargo.toml`
- `manifests/connectors/slack.toml`

## Files you must NOT touch
- Other connectors. The Slack APPROVAL channel (Stream
  D-slack-approvals) is a separate brief; don't touch its
  files.

## Acceptance criteria
- [ ] All 5 tools wired
- [ ] Bot token via env (`SLACK_BOT_TOKEN`); fail clearly if
      missing
- [ ] Integration test (gated on `SLACK_BOT_TOKEN`) hits a
      test channel
- [ ] `cargo clippy -p argo-mcp-connector-slack -- -D warnings`
      clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-C-slack`
- PR title: `mcp(connector): slack — post / list / read / dm / search (C-slack)`

## Notes
- Mirror C-gmail's shape if it landed first.
- Use `slack-morphism` or write thin reqwest calls; choose
  what's lighter on dependency footprint.
