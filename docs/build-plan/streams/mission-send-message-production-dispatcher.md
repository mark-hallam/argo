# mission-send-message-production-dispatcher — wire the production `send_message` dispatcher (P1 audit-finding-5)

**Owner**: claude. **Depends on**: none (consumes existing `argo-channels` surface).
**Authored against**: origin/main `6b432ec4` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-send-message-production-dispatcher`.
**Source**: 2026-05-17 read-only audit finding #5 (audit lines 113-127). Today the `send_message` tool is registered with its default dispatcher which returns `"send_message backend not wired"` at runtime — workers can't actually message anyone via this tool.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — the "teammate, not tool" + "works alongside humans" mission claims require workers to message humans via more than just approval-flow notifications.
- **Reliability** (#1) — the tool is advertised in `/meta/tools` but errors at dispatch time. Either wire it or remove it from the advertised set.

## Scope

P1. Build a production `MessageDispatcher` backed by `argo-channels`, then register `SendMessage::with_dispatcher(...)` at gateway boot.

### Production MessageDispatcher

```rust
// crates/argo-channels/src/message_dispatcher.rs (NEW)
pub struct ProductionMessageDispatcher {
    channels: Arc<ChannelRegistry>,   // existing argo-channels primitive
    audit:    Arc<AuditSink>,
    store:    Arc<SessionStore>,
}

impl ProductionMessageDispatcher {
    pub fn new(channels: ..., audit: ..., store: ...) -> Self { ... }
}

#[async_trait]
impl MessageDispatcher for ProductionMessageDispatcher {
    async fn send(&self, ctx: &MessageContext, body: &str, attachments: &[PathBuf])
        -> Result<SendOutcome, String>
    {
        // 1. Validate (tenant, channel, recipient) against capability gate
        // 2. Resolve channel via ChannelRegistry::get(ctx.channel)
        // 3. Send via channel.send(body, attachments)
        // 4. Emit AuditKind::MessageSent audit row with channel + recipient + outcome
        // 5. Return SendOutcome { message_id, channel, ts }
    }
}
```

Capability gate: `Capability::MessageSend { channel, recipient_scope }` (already exists per F-07). Per-tenant per-channel allowlist applies; reject with audit row if denied.

Idempotency: optional `message_id` parameter; if provided, check `outbound_message` table for prior send, return cached `SendOutcome` instead of double-send.

### Bootstrap wiring

```rust
// crates/argo-cli/src/handlers/gateway/boot.rs (or boot_router.rs)
let message_dispatcher = Arc::new(ProductionMessageDispatcher::new(
    Arc::clone(&channels),
    Arc::clone(&audit),
    Arc::clone(&store),
));
let tools = builtin_tools_with_runtime_and_message_dispatcher(
    ..., Some(message_dispatcher),
);
```

Extend `builtin_tools_with_runtime` to accept an optional `MessageDispatcher` and register `SendMessage::with_dispatcher(...)` when present.

### Migration 113: outbound_message idempotency table

```sql
CREATE TABLE outbound_message (
    message_id    TEXT PRIMARY KEY,
    tenant_id     BLOB NOT NULL,
    channel       TEXT NOT NULL,
    recipient     TEXT NOT NULL,
    body_hash     TEXT NOT NULL,           -- SHA-256 of body+attachments-manifest
    sent_at       INTEGER NOT NULL,
    outcome_json  TEXT NOT NULL
);
CREATE INDEX idx_outbound_tenant ON outbound_message(tenant_id, sent_at);
```

LOC: ~600.

## Files you may touch

- `crates/argo-channels/src/message_dispatcher.rs` (NEW) — production impl. ≤200 LOC.
- `crates/argo-channels/src/message_dispatcher_tests.rs` (NEW) — capability denial, idempotency, audit row. ≤120 LOC.
- `crates/argo-channels/src/lib.rs` — re-export. ≤5 modified LOC.
- `crates/argo-tool/src/tools/mod.rs` — extend `builtin_tools_with_runtime` to accept the dispatcher. ≤40 modified LOC.
- `crates/argo-tool/src/tools/send_message.rs` — read-only consumer; verify `SendMessage::with_dispatcher(...)` registration path works. No changes expected.
- `crates/argo-cli/src/handlers/gateway/boot_router.rs` (or `boot.rs` post-split) — wire the dispatcher at boot. ≤30 modified LOC.
- `crates/argo-store/migrations/113__outbound_message.sql` (NEW) — additive. ≤20 LOC.
- `crates/argo-store/src/outbound_message_query.rs` (NEW) — get + insert helpers. ≤60 LOC.
- `crates/argo-store/src/outbound_message_query_tests.rs` (NEW). ≤80 LOC.
- `crates/argo-store/src/lib.rs` — re-export. ≤4 modified LOC.

## Files you must NOT touch

- The `SendMessage` tool itself (read-only consumer of dispatcher).
- The `Capability::MessageSend` enum — consume only.
- Per-channel implementations (`argo-channels-slack`, `argo-channels-email`, etc.) — the dispatcher routes; channels handle send.
- The audit chain hash logic — emit audit rows via the standard `AuditSink` API.

## Acceptance criteria

- [ ] `ProductionMessageDispatcher::send(...)` routes to the correct channel via `ChannelRegistry`.
- [ ] Capability denial → returns `Err("not authorized: missing Capability::MessageSend{channel=...}")` AND emits `AuditKind::CapabilityDenied` row.
- [ ] Successful send → emits `AuditKind::MessageSent` row with `{ channel, recipient, message_id, outcome }`.
- [ ] Idempotency: same `message_id` twice → second call returns cached `SendOutcome` without re-sending.
- [ ] `argo-tool::SendMessage::with_dispatcher(...)` registered at boot when dispatcher is present.
- [ ] `/meta/tools` still lists `send_message`; runtime dispatch no longer returns "backend not wired".
- [ ] End-to-end test: worker dispatches `send_message`, message arrives via stub channel, audit row written.
- [ ] `cargo test -p argo-channels -p argo-tool -p argo-store -p argo-cli` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean for touched crates.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-send-message-production-dispatcher`
- Title: `feat(channels): production MessageDispatcher + boot wiring + idempotency (mission-send-message-production-dispatcher)`
- Worker verification: crate-scoped + workspace clippy + integration test per SWARM_PROTOCOL.md.

## Hidden constraints

- **`MessageContext` shape stays consistent** with the existing `SendMessage` tool's expectations — don't restructure the dispatcher interface.
- **Channels with deferred-v1.x status** (per send-message brief): the dispatcher routes to them if configured; if a deferred channel returns `NotImplemented`, the dispatcher returns a clear error rather than panicking.
- **Idempotency `body_hash`** prevents replay attacks but also catches accidental double-sends from worker retries.
- **`recipient_scope` capability check** is per-recipient, not per-channel. Worker can send to `user_X@example.com` via slack/email/etc. but only if the cap allows that recipient pattern.
- **Don't reuse the approval-notification path.** Approval notifications go through `worker_actions_notify.rs` and have their own audit/idempotency surface; production `send_message` is the general-purpose outbound, distinct from escalation.
- **No retry logic in this brief.** Failed send → return Err. Retry is a follow-up if needed (could reuse `codex-billing-webhook-resilience` pattern).
