# V1.2 — Discord MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Discord connector. Auth via bot token (`DISCORD_BOT_TOKEN`).

- Methods: `list_guilds`, `list_channels` (guild-scoped),
  `list_messages` (channel-scoped, with after/before),
  `get_message`, `send_message`, `add_reaction`,
  `list_members` (guild-scoped)
- Read autonomous-safe; send_message + add_reaction
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-discord/` mirroring
`argo-mcp-connector-asana`:
- `crates/argo-mcp-connector-discord/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-discord/src/{lib,main,auth,client,schemas,service}.rs` (NEW)
- `Cargo.toml` workspace — narrowly: add new member

NO CLI subcommand.

## Acceptance criteria
- [ ] All 7 methods callable
- [ ] Bot-token auth via DISCORD_BOT_TOKEN
- [ ] Approval-gate marks correct (read=false, send/react=true)
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-discord`
- PR title: `mcp: native Discord connector (codex-connector-discord)`
