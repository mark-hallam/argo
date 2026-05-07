# V1.4 — Discord approval channel

**Owner**: codex. **Depends on**: codex-connector-discord (#879)
merged.

## Scope

Discord MCP connector (#879) lets workers send messages. Missing
piece: a NotifyChannel that sends approval prompts to a Discord
channel and consumes button-press replies via webhook.

Pattern follows the existing slack-approvals (#783) /
sms-decision-route (#782) / push (#798) channels:
- `argo-channels::DiscordChannel implements NotifyChannel` — sends
  embed message + 2 buttons (Approve / Deny) via Discord REST.
- HMAC-signed callback URL with token-encoded action_id +
  decision verb; gateway route `/discord/decision` validates
  signature + records decision via existing approval pipeline.
- Workspace manifest: `[notify_channels.discord]` with
  channel_id + bot_token (or webhook URL).

## Files you may touch
- `crates/argo-channels/src/discord.rs` (NEW)
- `crates/argo-channels/src/lib.rs` — narrowly: re-export
- `crates/argo-channels/Cargo.toml` — narrowly: dep on
  argo-discord-client (or reqwest if shared client crate
  doesn't exist; mirror imessage pattern)
- `crates/argo-gateway/src/adapters/discord_decision.rs` (NEW)
  — HMAC-signed decision callback handler
- `crates/argo-gateway/src/router.rs` — narrowly: register
  `/discord/decision` route
- `crates/argo-channels/tests/discord_smoke.rs` (NEW)

## Acceptance criteria
- [ ] DiscordChannel.send() posts embed + 2 buttons via REST
- [ ] HMAC token validates: tampered tokens return 400
- [ ] Decision callback records via existing approval pipeline
- [ ] `argo doctor channels` recognizes Discord channel
      (extends doctor_channels.rs check pattern)
- [ ] 5 unit tests + 1 wire test (real HTTP behind
      DISCORD_BOT_TOKEN env)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-channel-discord-approvals`
- PR title: `channels: Discord approval channel + decision route (codex-channel-discord-approvals)`
