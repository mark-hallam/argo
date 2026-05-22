# mission-wire-slack-approval-roundtrip — Slack inbound webhook → approval state transition

**Owner**: claude. **Depends on**: nothing (Slack outbound notify shipped #931). **Tier M — Mission MVP critical path.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Mission claim: approvals route through chat, not a CLI. Slack outbound notify shipped
in #931, but inbound is missing — when a human clicks "Approve" in Slack, nothing happens
on our side. Without this, the approval roundtrip is half-built and operators must still
shell into the CLI. This brief closes the loop with a signed webhook that calls the
existing state-machine `approve` transition. To prove the trait is generic (not slack-only)
the brief also wires one extra channel (Email reply-to) through the same trait.

## Approach (concrete)

1. **NEW** `crates/argo-channels/src/slack_inbound.rs` (~250 LOC):
   - `SlackInbound` struct implementing a new `InboundChannel` trait (small, 2 methods:
     `verify_signature` + `decode_approval_token`).
   - HMAC-SHA256 signature verification per Slack docs, with replay protection via
     `X-Slack-Request-Timestamp` (reject > 5 min skew).
   - `decode_approval_token(payload) -> ArgoResult<ApprovalDecision { action_id, decision }>`
     using the existing approval-token signing format already used by outbound notify.
2. **NEW** `crates/argo-cli/src/handlers/gateway/routes_channels_slack_webhook.rs`
   (~200 LOC):
   - `POST /channels/slack/webhook` handler.
   - Verify signature (reject 401 on failure).
   - Decode token, look up action_id, call existing state-machine `approve` /
     `reject` transition.
   - Return 200 with the empty body Slack expects (`{}`).
3. **Modify** `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: add
   `pub mod routes_channels_slack_webhook;` line.
4. **Modify** `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: register the
   new route in the route table.
5. **Email parity (one extra channel to prove trait genericity)**: a stub
   `crates/argo-channels/src/email_inbound.rs` (~80 LOC) implementing the same
   `InboundChannel` trait for email reply-to. The full IMAP/SES wiring is deferred,
   but the trait impl + 1 unit test demonstrating signature verification + token
   decode against a recorded fixture proves the trait is not Slack-shaped.
6. **Capability**: declare `WebhookInbound::Slack` (extend the capability enum).
   **EXHAUSTIVE-MATCH amendment** applies for the new variant.
7. **Rustdoc**: every `pub` item carries doc + at least one example.

## Files you may touch

- `crates/argo-channels/src/slack_inbound.rs` (NEW, ≤ 300 LOC)
- `crates/argo-channels/src/slack_inbound_tests.rs` (NEW, ≤ 200 LOC) — 4 unit tests:
  valid signature accept, bad signature reject, stale timestamp reject, token decode.
- `crates/argo-channels/src/email_inbound.rs` (NEW, ≤ 100 LOC) — minimal trait impl.
- `crates/argo-channels/src/lib.rs` — narrowly: `pub mod` lines + `InboundChannel` trait export.
- `crates/argo-cli/src/handlers/gateway/routes_channels_slack_webhook.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_channels_slack_webhook_tests.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 pub mod line.
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: route registration.
- `crates/argo-cli/tests/slack_approval_roundtrip_e2e.rs` (NEW, ≤ 200 LOC) —
  1 integration test: action enters `pending` → recorded Slack webhook fixture POSTed
  to test gateway → action transitions to `approved` with no CLI call.
- `crates/argo-cli/tests/fixtures/slack_approval_webhook.json` (NEW) — recorded payload.
- `crates/argo-capability/src/lib.rs` — narrowly: `WebhookInbound::Slack` enum variant +
  the new arm in any existing match.

## Files you must NOT touch

- The state-machine `approve` / `reject` implementation — call the existing API only.
- Slack outbound notify code (#931) — already shipped.
- Any auth / RBAC primitive — webhook signature verification is its own auth path.

## Acceptance criteria

- [ ] HMAC signature verification rejects bad signatures with 401.
- [ ] Stale timestamp (> 5 min skew) rejected with 401.
- [ ] Valid webhook → action_id transitions `pending` → `approved` (or `rejected`).
- [ ] Email inbound trait impl + 1 unit test demonstrating trait genericity.
- [ ] 4+ unit tests + 1 integration roundtrip test using recorded fixture.
- [ ] Capability `WebhookInbound::Slack` declared; rustdoc on every pub item.
- [ ] `cargo clippy -p argo-channels -p argo-cli -p argo-capability --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `claude-mission-wire-slack-approval-roundtrip`
- PR title: `channels: Slack inbound approval webhook + Email trait parity (mission-wire-slack-approval-roundtrip)`
