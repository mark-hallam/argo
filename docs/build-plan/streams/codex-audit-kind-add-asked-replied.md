# codex-audit-kind-add-asked-replied — add ApprovalAsked + ApprovalReplied AuditKind variants (P0 audit-finding)

**Owner**: codex. **Sizing hint**: fast-mode candidate — small mechanical addition + emit hooks.
**Depends on**: none.
**Authored against**: origin/main `fa849bbb` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-audit-kind-add-asked-replied`.
**Source**: 2026-05-17 claude conductor audit finding P0-2. Mission claim 6 architecture diagram (`docs/skycot/architecture/21-audit-chain.md`) shows `ApprovalAsked` + `ApprovalReplied` as audit-row variants. They are NOT in `AuditKind` today — only `ApprovalGranted`, `ApprovalConsumed`, `ApprovalExpiredByTimeout` exist.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — audit chain forensics for incident response require both the ask AND the reply, not just the eventual grant.
- **Reliability** (#1) — claim 6 ("multi-channel approval gates") is partly paper without the ask/reply variants.

## Scope

P0. Three pieces:

### 1. Add the variants

In `crates/argo-audit/src/event.rs`:

```rust
pub enum AuditKind {
    // ... existing ...
    ApprovalAsked,        // NEW — emitted when worker_actions_notify::dispatch_notify fires
    ApprovalReplied,      // NEW — emitted when operator replies via slack/email/sms/voice/etc.
    ApprovalGranted,      // existing
    ApprovalConsumed,     // existing
    ApprovalExpiredByTimeout,  // existing
    // ...
}
```

Plus the matching:
- `Self::ApprovalAsked => "approval.asked"` in `as_str()`
- `"approval.asked" => Self::ApprovalAsked` in `from_str()`
- Add both to the `all()` enumeration

### 2. Emit `ApprovalAsked` at dispatch

In `crates/argo-cli/src/handlers/worker_actions_notify.rs` (or its split helpers post #1801 boot-router work):
- After successfully dispatching a notification via any channel (slack/email/sms/voice/discord/push/imessage/teams/matrix), emit `AuditSink::write(AuditKind::ApprovalAsked, payload)`.
- Payload: `{ action_id, worker_id, tenant_id, channel, recipient, risk_level, requested_at, expiry_ts }`.
- If multiple channels fire for one action (fallback chain), emit one row per channel.

### 3. Emit `ApprovalReplied` at inbound reply

In `crates/argo-cli/src/handlers/gateway/routes_channels_*.rs` for each channel's inbound webhook (`routes_channels_slack.rs`, `routes_channels_email.rs`, `routes_channels_voice.rs`, etc.):
- After HMAC verify + decode, emit `AuditKind::ApprovalReplied` BEFORE handing off to the gate.
- Payload: `{ action_id, channel, replier, reply_kind: "approve"|"deny", reply_body, replied_at }`.

LOC: ~120 (variants + 2 emit sites + tests).

## Files you may touch

- `crates/argo-audit/src/event.rs` — add 2 variants + 2 string mappings + add to `all()`. ≤25 modified LOC.
- `crates/argo-audit/src/event_tests.rs` (if exists) — assert round-trip for the 2 new variants. ≤30 modified LOC.
- `crates/argo-cli/src/handlers/worker_actions_notify.rs` — emit `ApprovalAsked` after dispatch. ≤30 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs` — emit `ApprovalReplied` on inbound. ≤15 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_channels_email.rs` — same. ≤15 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_channels_voice.rs` — same. ≤15 modified LOC.
- Other channel inbound handlers (sms, discord, push, imessage) — same pattern. ≤15 modified LOC each.
- `crates/argo-cli/src/handlers/worker_actions_notify_tests.rs` — extend to assert audit row emitted. ≤40 modified LOC.

## Files you must NOT touch

- The audit chain hash logic (`compute_chain_hash`) — read-only consumer.
- The gate decision logic in `worker_actions_auto_approval.rs` — emits `ApprovalGranted` already.
- Architecture diagrams — they already document the variants; this brief makes the code match.

## Acceptance criteria

- [ ] `AuditKind::ApprovalAsked` and `AuditKind::ApprovalReplied` exist + round-trip through `as_str`/`from_str`.
- [ ] Both appear in `AuditKind::all()` enumeration.
- [ ] `worker_actions_notify::dispatch_notify` emits `ApprovalAsked` per channel after success.
- [ ] At least 3 channel inbound handlers (slack + email + voice) emit `ApprovalReplied` on reply.
- [ ] End-to-end test: trigger an auto-approval Ask, verify `ApprovalAsked` row in audit chain; simulate operator approve, verify `ApprovalReplied` row.
- [ ] `cargo test -p argo-audit -p argo-cli` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean for touched crates.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-audit-kind-add-asked-replied`
- Title: `feat(audit): ApprovalAsked + ApprovalReplied variants + emit at dispatch and inbound (codex-audit-kind-add-asked-replied)`
- Worker verification: crate-scoped + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Canonical label strings:** `"approval.asked"` and `"approval.replied"` (snake_case after the dot, matching existing pattern).
- **Order matters in audit chain.** Emit `ApprovalAsked` AFTER channel-send succeeds (not before — failed-send shouldn't bloat chain with phantom asks).
- **One `ApprovalAsked` per channel fired** — fallback-chain that hits 3 channels emits 3 rows. This is intentional for forensic reconstruction.
- **`ApprovalReplied` is emitted on FIRST reply only** (first reply wins per existing channel-dispatch contract). Late replies emit a different kind (no new variant needed — existing log/discard path).
- **Closes audit finding P0-2 + P1-5** (chain shape + diagram-vs-code mismatch). Single PR closes both.
