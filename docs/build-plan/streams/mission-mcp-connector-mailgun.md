# Mission load-bearing wire — Mailgun MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-sendgrid/` as canonical SendGrid pattern; `crates/argo-mcp-connector-shared/`).
**Authored against**: origin/main `09673df7` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-mailgun`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #112 ("Add MCP connector SendGrid/Mailgun/Resend"). SendGrid ships today (`crates/argo-mcp-connector-sendgrid/`); Mailgun and Resend are the operator-requested fall-back providers (EU residency, lower volume tier). This brief lands Mailgun as the second outbound-email connector. Resend is queued as a future follow-up.

## Scope

NEW crate `crates/argo-mcp-connector-mailgun/` mirroring `argo-mcp-connector-sendgrid` shape (API-key auth, send-message endpoint, webhook signature verification):

```rust
// crates/argo-mcp-connector-mailgun/src/lib.rs
pub struct MailgunConnector { client: Client, api_key: String, domain: String, region: MailgunRegion }
pub enum MailgunRegion { Us, Eu }
impl MailgunConnector {
    pub fn new(api_key: String, domain: String, region: MailgunRegion) -> Self;
    pub async fn send_email(&self, msg: MailgunMessage) -> ArgoResult<MailgunSendResponse>;
    pub fn verify_webhook(&self, timestamp: &str, token: &str, signature: &str) -> bool;
}
pub struct MailgunMessage {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub reply_to: Option<String>,
    pub tags: Vec<String>,
}
pub struct MailgunSendResponse { pub message_id: String, pub message: String }
pub struct MailgunWebhookEvent {
    pub event: MailgunEventKind,  // delivered/opened/clicked/bounced/unsubscribed/complained/failed
    pub recipient: String,
    pub message_id: String,
    pub timestamp: f64,
}
```

Webhook verification: Mailgun signs webhooks with `HMAC-SHA256(timestamp + token, api_key)`. Constant-time compare against `signature`.

This is NOT in scope: template variables (Mailgun's `t:variables` feature); inbound parse routes; mailing list management; suppression list CRUD; Resend (separate follow-up brief); gateway webhook-dispatch wiring for inbound replies (separate follow-up brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-mailgun/` (NEW crate, ≤ 800 LOC total)
  - `Cargo.toml` (NEW, ≤ 30 LOC)
  - `src/lib.rs` (NEW, ≤ 120 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 300 LOC) — `MailgunConnector` + POST to `https://api.{region}.mailgun.net/v3/{domain}/messages`
  - `src/types.rs` (NEW, ≤ 150 LOC) — `MailgunMessage`, `MailgunSendResponse`, `MailgunWebhookEvent`, serde derives
  - `src/webhook.rs` (NEW, ≤ 100 LOC) — HMAC verification
  - `src/client_tests.rs` (NEW, sibling, ≤ 250 LOC) — 5 unit tests w/ wiremock: send-success, send-401, webhook-verify-pass, webhook-verify-fail, EU region URL
  - `tests/mailgun_smoke.rs` (NEW, ≤ 150 LOC) — gated by `ARGO_MAILGUN_TEST_KEY` + `ARGO_MAILGUN_TEST_DOMAIN` + `ARGO_MAILGUN_TEST_RECIPIENT`; sends a real email, asserts 200 + message_id present
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-mailgun` to `members`

## Files you must NOT touch

- `crates/argo-mcp-connector-sendgrid/**` — sibling stable, mirror its shape, don't modify
- Any other connector crate
- `crates/argo-cli/**` — gateway/dispatch wiring is a follow-up
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-mailgun/` crate compiles standalone
- [ ] `MailgunConnector::new` constructs with api_key, domain, region
- [ ] `send_email` POSTs `application/x-www-form-urlencoded` body to `https://api.{us|eu}.mailgun.net/v3/{domain}/messages` with Basic auth (user `api`, pass `<api_key>`)
- [ ] Region routing: `Us` → `api.mailgun.net`; `Eu` → `api.eu.mailgun.net`
- [ ] `verify_webhook` returns true iff HMAC-SHA256(timestamp + token, api_key) matches signature in constant time
- [ ] `MailgunWebhookEvent::event` deserializes all 7 standard event names
- [ ] 5 unit tests with wiremock
- [ ] Integration smoke gated by 3 env vars; skipped silently when absent
- [ ] `cargo clippy -p argo-mcp-connector-mailgun --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-mailgun` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-mailgun`
- PR title: `mcp-connector: Mailgun outbound + webhook-verify connector (mission-mcp-connector-mailgun)`

## Notes for the implementer

- Mailgun auth is HTTP Basic: username `api`, password the API key. Some clients call this the "private API key" (`key-...` prefix). Don't confuse with public-validation key.
- EU region is operator-driven (data residency); region affects only base URL.
- Webhook timestamp tolerance: reject if `|now - timestamp| > 15 min` (replay protection).
- Body content-type for `send_email` is `application/x-www-form-urlencoded`, NOT JSON. Recipients are repeated keys: `to=a@x&to=b@y`.
- Retriable errors: 429, 5xx. Reuse `argo-mcp-connector-shared::retry` if it exists; otherwise leave to caller.
