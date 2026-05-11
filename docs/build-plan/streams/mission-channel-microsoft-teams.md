# Mission load-bearing wire — Microsoft Teams channel

**Owner**: claude. **Depends on**: existing `argo-channels` channel-impl pattern (verify via `ls crates/argo-channels/src/` — pattern is per-channel module like `slack.rs`, `discord.rs`).
**Authored against**: origin/main `47d39166` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-channel-microsoft-teams`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #125 ("Add Microsoft Teams channel"). **Tier A (tool).** Channel parity with the existing Slack / Discord / WhatsApp / Voice / Email / SMS surfaces. Microsoft Teams covers the enterprise-IT customer segment that won't use Slack — same approval-routing primitive, different transport.

## Scope

NEW module `crates/argo-channels/src/teams.rs` (≤ 280 LOC) implementing the existing `NotifyChannel` trait. Uses Microsoft Graph API's `chats/messages` endpoint w/ Azure-AD app-only auth (client_credentials grant) — NOT the older incoming-webhook approach (deprecated in late 2024).

```rust
pub struct TeamsChannel {
    client: Client,
    tenant_id: String,
    client_id: String,
    client_secret: SecretString,
    default_team_id: Option<String>,
    default_channel_id: Option<String>,
    token_cache: RwLock<Option<CachedToken>>, // refresh on 401 or 5min-before-expiry
}

impl NotifyChannel for TeamsChannel {
    fn name(&self) -> &'static str { "teams" }
    async fn send(&self, msg: &OutboundMessage) -> ArgoResult<SendReceipt>;
    async fn supports(&self) -> ChannelCapabilities { /* rich, threading, mentions, attachments */ }
}
```

Message-shape mapping:
- `OutboundMessage.text` → Teams `body.content` (HTML rich-text; markdown converted via existing `argo-channels::markdown_to_html` if it exists, else minimal `<p>` wrap)
- `OutboundMessage.attachments` → Teams adaptive-card attachments (one per attachment)
- `OutboundMessage.thread_id` → reply via `chats/<thread_id>/messages` instead of new-chat
- Mentions: `@user_email` in text → Teams adaptive-card `mentions` array w/ resolved AAD user id (Graph `/users/<email>` lookup, cached 1h)

Inbound (webhook) is intentionally OUT of scope for v1 — operators using Teams will dispatch outbound only. Inbound replies are a Bot Framework Connector concern, separate brief once a vertical actually needs Teams two-way.

NEW `[channels.teams]` config block (`crates/argo-config/src/sections.rs`):
- `enabled: bool = false`
- `tenant_id: String` (Azure AD tenant)
- `client_id: String`
- `client_secret_env: String = "ARGO_TEAMS_CLIENT_SECRET"` (env-indirect to avoid plaintext-in-config)
- `default_team_id: Option<String>` (when message has no explicit team)
- `default_channel_id: Option<String>`
- `markdown_to_html: bool = true`

This is NOT in scope: inbound Bot Framework Connector wiring (separate brief); MS Teams native app submission / certified-bot listing (separate process, not code); voice/video call placement (separate brief — Teams meetings API); shared-channel / cross-tenant message routing; Power Automate flow integration; Teams "card actions" (button-clicks on adaptive cards become webhook events — separate brief once inbound lands).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-channels/src/teams.rs` (NEW, ≤ 280 LOC)
- `crates/argo-channels/src/teams_tests.rs` (NEW, sibling, ≤ 280 LOC) — 7 unit tests:
  - send-to-default-channel-when-no-explicit-team
  - send-to-explicit-team-channel
  - reply-to-thread-uses-chats-endpoint
  - mention-resolution-cached-second-call-no-hit (mock graph users endpoint)
  - token-refresh-on-401-then-retries-once
  - token-refresh-on-5min-before-expiry-proactively
  - attachment-becomes-adaptive-card
- `crates/argo-channels/src/lib.rs` — narrowly scoped: register `pub mod teams;` + `pub use teams::TeamsChannel`
- `crates/argo-channels/Cargo.toml` — narrowly scoped: ADD any needed deps (likely already covered by existing reqwest+serde+tokio+tracing — verify first)
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[channels.teams]` section
- `crates/argo-config/src/defaults.rs` — narrowly scoped
- `crates/argo-channels/tests/teams_smoke.rs` (NEW, ≤ 200 LOC) — integration smoke gated by `ARGO_TEAMS_TEST_*` env vars (tenant_id, client_id, client_secret, target_channel): send one test message + assert receipt has the expected `chats/<id>/messages` URL shape

## Files you must NOT touch

- `crates/argo-channels/src/{slack,email,sms,voice,whatsapp,discord,pagerduty}.rs` — sibling stable
- `crates/argo-store/migrations/**` — no schema
- Vertical content / frontend

## Acceptance criteria

- [ ] `TeamsChannel::new` constructs from `[channels.teams]` config + env-loaded secret
- [ ] Send to default channel works when `OutboundMessage` has no explicit `team_id` + `channel_id`
- [ ] Send to explicit team/channel overrides default
- [ ] Token cache: second call within 1h does NOT re-hit OAuth token endpoint; verified by wiremock expect-no-call
- [ ] 401 on a send triggers token refresh + ONE retry; failed retry returns typed `ArgoError::Channel`
- [ ] @-mention resolves email→AAD-user-id via `/users/<email>` Graph endpoint, cached 1h
- [ ] Markdown-to-HTML conversion preserves bold/italic/code/links (use existing `markdown_to_html` if present)
- [ ] 7 unit tests + 1 integration smoke (env-gated)
- [ ] `cargo clippy -p argo-channels -p argo-config --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-channels -p argo-config` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-channel-microsoft-teams`
- PR title: `channels: Microsoft Teams (Graph API app-only, send + mentions + thread replies) (mission-channel-microsoft-teams)`

## Notes for the implementer

- Use Microsoft Graph v1.0, NOT the older `https://outlook.office.com/webhook/...` deprecated incoming-webhook surface. Graph API is forward-compatible w/ Teams' adaptive-card schema.
- OAuth scope: `https://graph.microsoft.com/.default` (app-only / client-credentials grant; no user delegation).
- Token endpoint: `https://login.microsoftonline.com/<tenant_id>/oauth2/v2.0/token` — POST `grant_type=client_credentials&client_id=...&client_secret=...&scope=https://graph.microsoft.com/.default`.
- Message-send endpoint: `POST https://graph.microsoft.com/v1.0/teams/<team_id>/channels/<channel_id>/messages`. Thread replies: `POST .../messages/<thread_id>/replies`.
- Adaptive-card schema for attachments: use `application/vnd.microsoft.card.adaptive` content-type, version 1.5 (broad client support). Don't try to use `MessageCard` (legacy webhook format).
- Mentions: Graph requires both `<at id="0">Display Name</at>` in HTML body AND a parallel `mentions: [{ id: 0, mentionText: "Display Name", mentioned: { user: { id: "<aad-user-id>", displayName: "..." } } }]` array. Get this wrong and the @-mention shows as plain text.
- Client-secret rotation: env-indirect (`ARGO_TEAMS_CLIENT_SECRET`) lets operators rotate without restart-and-edit-config; document this in module docs.
- Mirror the existing `argo-channels::discord` module for shape — both are HTTP-out-only channels with token-based auth. Same OutboundMessage→native-shape mapping pattern, same `NotifyChannel` trait impl.
