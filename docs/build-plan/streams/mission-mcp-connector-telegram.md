# Mission load-bearing wire — Telegram MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (verified at `crates/argo-mcp-connector-discord/`, `crates/argo-mcp-connector-shared/`).
**Authored against**: origin/main `09673df7` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-telegram`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #124 ("Add Telegram channel"). Claim 6 (multi-channel approvals) ships Slack, Discord, iMessage, WhatsApp, Email, Voice, SMS today; Telegram is the highest-frequency request from EU/APAC operator pilots. A bot-token connector (no OAuth) is the smallest viable addition.

## Scope

NEW crate `crates/argo-mcp-connector-telegram/` mirroring `argo-mcp-connector-discord` shape (single-purpose connector, bot-token auth, send-message + receive-update endpoints):

```rust
// crates/argo-mcp-connector-telegram/src/lib.rs
pub struct TelegramConnector { client: Client, bot_token: String }
impl TelegramConnector {
    pub fn new(bot_token: String) -> Self;
    pub async fn send_message(&self, chat_id: i64, text: &str) -> ArgoResult<TelegramMessage>;
    pub async fn send_reply_keyboard(&self, chat_id: i64, text: &str, buttons: Vec<Vec<String>>) -> ArgoResult<TelegramMessage>;
    pub async fn answer_callback_query(&self, query_id: &str, text: Option<&str>) -> ArgoResult<()>;
}
pub struct TelegramMessage { pub message_id: i64, pub chat_id: i64, pub date: i64 }
pub struct TelegramWebhookUpdate { pub update_id: i64, pub message: Option<TelegramMessage>, pub callback_query: Option<TelegramCallbackQuery> }
pub struct TelegramCallbackQuery { pub id: String, pub from_user_id: i64, pub data: Option<String> }
```

Webhook verification: Telegram uses `X-Telegram-Bot-Api-Secret-Token` header for the operator-provided secret (set at webhook registration). The connector exposes `verify_webhook_secret(headers, expected) -> bool`.

This is NOT in scope: in-line keyboards beyond reply-keyboard (separate brief); media upload (`sendPhoto`, `sendDocument`); polling mode (operators must use webhooks); user-bot OAuth (different API entirely); gateway webhook-dispatch wiring (mirror `is_imessage_webhook` follow-up brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-telegram/` (NEW crate, ≤ 800 LOC total across all files)
  - `Cargo.toml` (NEW, ≤ 30 LOC)
  - `src/lib.rs` (NEW, ≤ 120 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 300 LOC) — `TelegramConnector` + HTTPS calls (`https://api.telegram.org/bot<TOKEN>/`)
  - `src/types.rs` (NEW, ≤ 150 LOC) — `TelegramMessage`, `TelegramWebhookUpdate`, `TelegramCallbackQuery`, serde derives
  - `src/webhook.rs` (NEW, ≤ 80 LOC) — `verify_webhook_secret`
  - `src/client_tests.rs` (NEW, sibling, ≤ 250 LOC) — 4 unit tests w/ wiremock
  - `tests/telegram_smoke.rs` (NEW, ≤ 150 LOC) — gated by `ARGO_TELEGRAM_TEST_TOKEN` + `ARGO_TELEGRAM_TEST_CHAT_ID`; sends a real message, polls update, asserts byte-round-trip
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-telegram` to `members`

## Files you must NOT touch

- Any other connector crate
- `crates/argo-cli/src/handlers/gateway/routes_channels*.rs` — webhook dispatch wiring is a follow-up brief
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-telegram/` crate compiles standalone
- [ ] `TelegramConnector::send_message` posts to `bot{TOKEN}/sendMessage` with `chat_id` + `text`
- [ ] `TelegramConnector::send_reply_keyboard` includes `reply_markup` JSON
- [ ] `TelegramConnector::answer_callback_query` POSTs to `bot{TOKEN}/answerCallbackQuery`
- [ ] `verify_webhook_secret` returns true iff `X-Telegram-Bot-Api-Secret-Token` matches
- [ ] 4 unit tests with wiremock: send-success, send-401, callback-answer, secret-match
- [ ] Integration smoke gated by `ARGO_TELEGRAM_TEST_TOKEN`; skipped silently when absent
- [ ] `cargo clippy -p argo-mcp-connector-telegram --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-telegram` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-telegram`
- PR title: `mcp-connector: Telegram bot-token connector (mission-mcp-connector-telegram)`

## Notes for the implementer

- Telegram Bot API is plain HTTPS — no SDK needed. `reqwest::Client` with JSON body is sufficient.
- Bot token format: `<bot_id>:<auth_hash>` (e.g. `123456:ABC-DEF...`). Treat as secret; do NOT log raw token.
- `X-Telegram-Bot-Api-Secret-Token` header is the operator's shared secret registered at `setWebhook` time. Constant-time comparison.
- Retriable errors: 429 (Too Many Requests with `retry_after`), 5xx. Use existing `argo-mcp-connector-shared::retry` if it exists; otherwise leave retry to caller (separate brief).
- The webhook-dispatch wiring (analogous to `is_imessage_webhook`) is intentionally deferred to a follow-up brief once this primitive lands, to keep this PR scoped.
