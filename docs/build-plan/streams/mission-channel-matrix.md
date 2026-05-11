# Mission load-bearing wire — Matrix channel

**Owner**: claude. **Depends on**: existing `argo-channels` channel-impl pattern.
**Authored against**: origin/main `47d39166` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-channel-matrix`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #127 ("Add Matrix channel"). **Tier A (tool).** Matrix is the federated, open-protocol chat surface — Element, FluffyChat, Beeper bridges, plus the privacy-conscious self-hosted home-server segment. Channel parity for operators who refuse Slack/Discord/Teams on principle. Plus the existing signal connector pairs well with Matrix for E2E-encrypted approval routing.

## Scope

NEW module `crates/argo-channels/src/matrix.rs` (≤ 260 LOC) implementing the existing `NotifyChannel` trait against the Matrix Client-Server API r0.6.1 (latest stable).

```rust
pub struct MatrixChannel {
    client: Client,
    homeserver_url: String,       // e.g. "https://matrix.org" or self-hosted
    access_token: SecretString,   // long-lived via /_matrix/client/r0/login (password) or app-service-style token
    user_id: String,              // "@argo:matrix.example.com"
    default_room_id: Option<String>, // "!abc123:matrix.example.com"
    txn_counter: AtomicU64,       // monotonic for idempotent PUT
}

impl NotifyChannel for MatrixChannel {
    fn name(&self) -> &'static str { "matrix" }
    async fn send(&self, msg: &OutboundMessage) -> ArgoResult<SendReceipt>;
    async fn supports(&self) -> ChannelCapabilities { /* threading=true, mentions=true, e2e_hint=false_v1 */ }
}
```

Message-send: `PUT /_matrix/client/r0/rooms/<room>/send/m.room.message/<txn_id>` w/ body `{ msgtype: "m.text", body: "...", format: "org.matrix.custom.html", formatted_body: "<html>" }`. `txn_id` is monotonic from `txn_counter` — idempotent retries don't double-send.

Threading: Matrix uses `m.thread` relation (MSC3440). When `OutboundMessage.thread_id` is `Some(event_id)`, include `m.relates_to: { rel_type: "m.thread", event_id: "..." }` in the message body.

Mentions: `m.mentions` field (MSC3952) — `{ user_ids: ["@alice:matrix.org"] }`. Element/most modern clients honor this; older clients see the inline `@alice` text.

NEW `[channels.matrix]` config block:
- `enabled: bool = false`
- `homeserver_url: String`  (e.g. `"https://matrix.org"`)
- `user_id: String`  (`"@argo-bot:matrix.example.com"`)
- `access_token_env: String = "ARGO_MATRIX_ACCESS_TOKEN"`
- `default_room_id: Option<String>`

E2E (end-to-end encryption) is OUT of scope for v1 — Matrix's E2E uses Olm/Megolm which requires per-device keys, persistent session store, and device verification UX. Send-only-cleartext is the v1 surface; document this limitation explicitly. (If an operator needs E2E for approval routing, that's a follow-up brief requiring matrix-rust-sdk integration, not the lightweight reqwest-based approach taken here.)

This is NOT in scope: E2E encryption (separate brief — substantial); receiving messages / sync streaming (separate, requires `/_matrix/client/r0/sync` long-poll loop); room creation / invite handling; SSO / OIDC login flow (only static long-lived access token in v1); voice/video / call placement (separate); federation bridge configuration (operator concern, not code).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-channels/src/matrix.rs` (NEW, ≤ 260 LOC)
- `crates/argo-channels/src/matrix_tests.rs` (NEW, sibling, ≤ 260 LOC) — 7 unit tests:
  - send-to-default-room
  - send-to-explicit-room-overrides-default
  - txn-id-monotonic-across-sends (no collisions)
  - retry-with-same-txn-id-is-idempotent (mock 200 on second call, expect server received same txn_id)
  - thread-reply-includes-m-relates-to
  - mention-includes-m-mentions-array
  - html-formatted-body-set-when-text-has-markdown
- `crates/argo-channels/src/lib.rs` — narrowly scoped: register `pub mod matrix;`
- `crates/argo-channels/Cargo.toml` — narrowly scoped if needed
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[channels.matrix]`
- `crates/argo-config/src/defaults.rs` — narrowly scoped
- `crates/argo-channels/tests/matrix_smoke.rs` (NEW, ≤ 180 LOC) — env-gated smoke against matrix.org or self-hosted homeserver (`ARGO_MATRIX_TEST_*`)

## Files you must NOT touch

- Other channel impls (`slack.rs`, `discord.rs`, `teams.rs` etc.) — sibling stable
- `crates/argo-store/migrations/**` — no schema
- Vertical / frontend

## Acceptance criteria

- [ ] `MatrixChannel::new` constructs from `[channels.matrix]` + env-loaded access token
- [ ] Send to default room works when `OutboundMessage` has no explicit `room_id`
- [ ] Send to explicit room overrides default
- [ ] `txn_counter` is monotonic — no two sends use same `txn_id` even under concurrency (Atomic order)
- [ ] Retry of a failed send with the same `txn_id` is treated as idempotent by the server (matrix property); verified via wiremock expect-no-duplicate
- [ ] `OutboundMessage.thread_id = Some(event_id)` produces an `m.thread`-relation message
- [ ] `OutboundMessage.mentions = ["alice@matrix.org"]` produces `m.mentions: { user_ids: ["@alice:matrix.org"] }`
- [ ] Markdown text produces both plain `body` and HTML `formatted_body`
- [ ] Module docs explicitly state "v1 is cleartext-only; E2E is a follow-up brief"
- [ ] 7 unit tests + 1 integration smoke (env-gated)
- [ ] `cargo clippy -p argo-channels -p argo-config --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-channels -p argo-config` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-channel-matrix`
- PR title: `channels: Matrix (Client-Server API send + threads + mentions, cleartext) (mission-channel-matrix)`

## Notes for the implementer

- Don't use `matrix-rust-sdk` — it's a heavy dep (~150 transitive crates) and pulls in OLM/Megolm crypto we don't need for cleartext send. Stay reqwest+serde lightweight.
- Get the user_id format right: `@local:domain` — both colon and `@` are part of the ID, NOT optional.
- Room IDs start with `!` (NOT `#` — that's the human-readable alias which needs resolution). Document this in config docs and reject malformed IDs at config-load time.
- `txn_counter` should persist across restarts if you can — Matrix's idempotency property is "same txn_id + same room = same event". If counter resets to 0 on restart, the FIRST post-restart send could be a duplicate of a pre-restart send. Acceptable for v1 (just document); ideal-state is persist in store but that's a separate brief.
- `m.mentions` is MSC3952; clients without support fall back to grepping the `body` for `@local:domain` strings. That works fine — clients without `m.mentions` support are pre-2024.
- Mirror `crates/argo-channels/src/discord.rs` for shape — same long-token-auth HTTP-out-only pattern. Different message-format ceremony, same trait surface.
- The integration smoke can use matrix.org as the homeserver if the operator creates a test bot user there. Free, public, federated — no self-hosting cost to verify the brief.
