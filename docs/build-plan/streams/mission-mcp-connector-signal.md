# Mission load-bearing wire — Signal MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-discord/` as canonical bot-style messaging pattern; `crates/argo-mcp-connector-shared/` retry primitive); existing iMessage / WhatsApp / Slack / Discord channels in the multi-channel approval surface.
**Authored against**: origin/main `55047304` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-signal`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #126 ("Add Signal channel"). Signal is the operator-requested high-privacy channel — sister to iMessage (per-device) but cross-platform and E2EE by default. Several enterprise pilots want Signal as the approval channel for HIPAA-adjacent workflows. signal-cli is the canonical Linux/macOS bridge (a Java client wrapping libsignal); this connector wraps signal-cli's local HTTP daemon mode.

## Scope

NEW crate `crates/argo-mcp-connector-signal/` mirroring `argo-mcp-connector-discord` shape (local daemon HTTP auth, send-message + receive-update endpoints):

```rust
// crates/argo-mcp-connector-signal/src/lib.rs
pub struct SignalConnector { client: Client, daemon_url: String, account: String }
impl SignalConnector {
    pub fn new(daemon_url: String, account: String) -> Self;  // daemon_url = "http://127.0.0.1:8080" typical
    pub async fn send_message(&self, recipient: &str, text: &str) -> ArgoResult<SignalMessageReceipt>;
    pub async fn send_group_message(&self, group_id: &str, text: &str) -> ArgoResult<SignalMessageReceipt>;
    pub async fn receive(&self) -> ArgoResult<Vec<SignalIncomingMessage>>;
    pub async fn list_groups(&self) -> ArgoResult<Vec<SignalGroup>>;
}
pub struct SignalMessageReceipt { pub timestamp: i64 }
pub struct SignalIncomingMessage {
    pub envelope_timestamp: i64,
    pub source_number: String,
    pub group_id: Option<String>,
    pub body: Option<String>,
    pub attachments: Vec<SignalAttachmentMeta>,
}
pub struct SignalGroup { pub id: String, pub name: String, pub members: Vec<String> }
pub struct SignalAttachmentMeta { pub id: String, pub content_type: String, pub size: u64 }
```

Daemon: signal-cli runs as a local subprocess/systemd unit at `127.0.0.1:8080` (operator-configured). The connector ONLY speaks to the loopback daemon; it never talks to Signal infrastructure directly. Setup runbook is a separate doc-brief follow-up.

This is NOT in scope: bundled signal-cli install (operator runs it); group create/admin endpoints (only list + send-to); attachment upload (text only for v1); registration / linking flow (operator does this once via `signal-cli link`); webhook-style push from signal-cli (poll-only); gateway dispatcher wiring (separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-signal/` (NEW crate, ≤ 800 LOC total)
  - `Cargo.toml` (NEW, ≤ 30 LOC)
  - `src/lib.rs` (NEW, ≤ 120 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 300 LOC) — `SignalConnector` + REST calls to signal-cli JSON-RPC at `daemon_url/api/v1/`
  - `src/types.rs` (NEW, ≤ 200 LOC) — DTOs + serde derives
  - `src/client_tests.rs` (NEW, sibling, ≤ 250 LOC) — 5 unit tests w/ wiremock: send-success, send-to-group, receive-empty, receive-with-attachment-meta, daemon-unreachable typed-error
  - `tests/signal_smoke.rs` (NEW, ≤ 150 LOC) — gated by `ARGO_SIGNAL_DAEMON_URL` + `ARGO_SIGNAL_ACCOUNT` + `ARGO_SIGNAL_TEST_RECIPIENT`; sends a real message, polls receive, asserts body roundtrip
  - `README.md` (NEW, ≤ 150 LOC) — signal-cli install pointer + daemon-mode startup snippet
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-signal` to `members`

## Files you must NOT touch

- Any other connector crate
- `crates/argo-cli/src/handlers/gateway/routes_channels*.rs` — webhook dispatch wiring is a follow-up
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-signal/` crate compiles standalone
- [ ] `SignalConnector::new` constructs with daemon_url + account
- [ ] `send_message` POSTs to `{daemon_url}/api/v1/send` with `{"number": "<account>", "recipient": "<recipient>", "message": "<text>"}`
- [ ] `send_group_message` POSTs with `groupId` field instead of `recipient`
- [ ] `receive` GETs `{daemon_url}/api/v1/receive/<account>` — returns deserialized envelope list
- [ ] `list_groups` GETs `{daemon_url}/api/v1/groups/<account>`
- [ ] Daemon-unreachable returns typed `ArgoError::Provider` with hint "ensure signal-cli daemon mode is running at <url>"
- [ ] 5 unit tests with wiremock; integration smoke gated by 3 env vars
- [ ] `cargo clippy -p argo-mcp-connector-signal --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-signal` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-signal`
- PR title: `mcp-connector: Signal via signal-cli daemon (send / receive / groups) (mission-mcp-connector-signal)`

## Notes for the implementer

- The signal-cli daemon mode exposes a JSON-RPC-over-HTTP API (canonical reference: signal-cli's `rest-api` docs). All operations are POST or GET with JSON body/response.
- `account` is the signal-cli-registered phone number with country code (e.g. `+15551234`). Validate that it starts with `+` and contains only digits.
- Daemon timeout: default 30s for `send`/`receive`; long-poll receive is intentionally NOT exposed in v1 (caller polls every N seconds).
- `Body` is `Option<String>` in `SignalIncomingMessage` because some envelopes are receipts/typing-indicators with no body — preserve nullability.
- The integration smoke requires real signal-cli setup; operators must run `signal-cli -a +XXX daemon --http 0.0.0.0:8080` before the test. Document this in the README, not in the test itself.
