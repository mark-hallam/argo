# V1.4 — Discord approval channel (rewrite v2 — correct gateway crate paths)

**Owner**: codex. **Depends on**: codex-connector-discord (#953 re-author MERGED — original #879 was closed).

> **REWRITE NOTE (tick 363 by conductor)**: original brief pointed to
> `crates/argo-gateway/src/router.rs` and `crates/argo-gateway/src/adapters/discord_decision.rs`
> — wrong crate. `argo-gateway` is the chat-platform **inbound adapters**
> crate (Slack/Discord/Telegram via `Adapter` trait + `GatewayRouter`
> session-routing), with NO HTTP server inside. The actual HTTP routes
> serving `argo gateway start` at port 7090 live in
> `crates/argo-cli/src/handlers/gateway/`. This rewrite redirects to the
> correct paths and mirrors the existing `routes_channels_slack.rs`
> pattern.

## Scope

Discord MCP connector (#953) lets workers send messages. Missing piece: a NotifyChannel that sends approval prompts to a Discord channel and consumes button-press replies via webhook.

Pattern follows the existing **slack-approvals** flow exactly:
- `argo-channels::DiscordChannel` implements NotifyChannel — sends embed message + 2 buttons (Approve / Deny) via Discord REST.
- HMAC-signed callback URL with token-encoded action_id + decision verb.
- Gateway route `POST /channels/discord/decision` validates signature + records decision via existing approval pipeline.
- Workspace manifest: `[notify_channels.discord]` with channel_id + bot_token (or webhook URL).

## Approach (concrete)

The reference implementation is `slack` end-to-end:
- `crates/argo-channels/src/slack.rs` (280 LOC) — channel impl + HMAC + send
- `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs` (137 LOC) — HTTP decision route handler
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs` (190 LOC) — central dispatcher; register the new discord route here

Mirror this pattern for Discord. Discord's bot REST is well-documented (interactions endpoint); use `reqwest` (already a workspace dep, used by #953).

## Files you may touch

- `crates/argo-channels/src/discord.rs` (NEW, ≤ 300 LOC) — DiscordChannel struct + NotifyChannel impl + HMAC token logic. Mirror `slack.rs` shape.
- `crates/argo-channels/src/lib.rs` — narrowly: 1 line `pub mod discord;` + re-export `pub use discord::DiscordChannel;`
- `crates/argo-channels/Cargo.toml` — narrowly: add `reqwest`, `hmac`, `sha2`, `subtle` deps if not already present (slack.rs uses them — likely already there)
- `crates/argo-cli/src/handlers/gateway/routes_channels_discord.rs` (NEW, ≤ 200 LOC) — `POST /channels/discord/decision` handler. Mirror `routes_channels_slack.rs` shape: parse signed token, validate HMAC, route to existing approval-decision pipeline.
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs` — narrowly: 1 dispatch arm registering `/channels/discord/decision`
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod routes_channels_discord;` line
- `crates/argo-cli/src/handlers/doctor_channels.rs` — narrowly: 1 check arm for Discord (mirror existing slack check)
- `crates/argo-channels/tests/discord_smoke.rs` (NEW, ≤ 200 LOC) — round-trip integration: send a fake approval → read decision back via mock HTTP
- Per BLANKET CLI ALLOWLIST: `crates/argo-cli/src/cli.rs`, `crates/argo-cli/src/main.rs`, `crates/argo-cli/src/handlers/mod.rs` allowed for any small wiring

## Files you must NOT touch

- `crates/argo-gateway/` — the chat-platform adapters crate. NO HTTP server inside; original brief mistakenly pointed here. This rewrite does NOT touch that crate.
- `crates/argo-mcp-connector-discord/` — that's #953's bot-API connector for outbound message sending. This brief is the approval-side: button taps + decision callback. Different layer.
- The 5 existing `routes_channels_*.rs` siblings — keep them untouched; this brief adds a new sibling, doesn't modify the others.
- `worker_actions_notify.rs` — that's `mission-notify-channels-rest-of-pack`'s domain (currently CLAIMED by codex). Stay out.

## Acceptance criteria

- [ ] `DiscordChannel.send()` posts embed + 2 buttons via Discord bot REST (using `DISCORD_BOT_TOKEN` + channel_id from `[notify_channels.discord]` manifest section)
- [ ] HMAC-signed decision callback URL: `POST /channels/discord/decision` validates signature; tampered tokens return 400
- [ ] Decision callback records via existing approval pipeline (same flow as slack-approvals)
- [ ] `argo doctor channels` recognizes Discord channel (1 new arm in `doctor_channels.rs`)
- [ ] 5 unit tests + 1 integration smoke (gated on `DISCORD_BOT_TOKEN` env — skip cleanly when unset, mirror slack pattern)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `codex-channel-discord-approvals`
- PR title: `channels: Discord approval channel + decision route (codex-channel-discord-approvals)`

## Notes for the implementer

- This is a **mirror** of slack-approvals + slack-decision-route — read both `slack.rs` (280 LOC) and `routes_channels_slack.rs` (137 LOC) end-to-end before starting. Your two files should be ~95% similar in structure.
- Discord button interactions arrive as webhook POSTs to your `/channels/discord/decision` endpoint with a signed payload. Discord's interaction-endpoint docs explain the verification (Ed25519 signature in headers); you can use the simpler HMAC-token-in-URL pattern that slack uses if Discord allows custom callback URLs (it does, via the bot's interaction URL config).
- The chat-platform `Adapter` trait in `argo-gateway` is for **inbound** chat conversations (worker receives a Slack DM, generates a response). It is unrelated to the **approval-decision** flow handled here. Don't confuse the two.
- 4 prior tabs have hit the `argo-gateway` naming confusion (this brief, web-dashboard-mvp, multi-host-orchestrator-design, gateway-bearer-auth original). The lesson: the HTTP gateway lives in `crates/argo-cli/src/handlers/gateway/`. Always.
- Mission claim 6 ("multi-channel approvals") gains another channel after this lands. Combined with `mission-notify-channels-rest-of-pack` (also currently CLAIMED by codex), claim 6 reaches ~95% delivered.
