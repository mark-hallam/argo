# Stream D — Email approval channel (Postmark)

**Owner**: codex. **No dependencies.** This brief defines the
`NotifyChannel` trait that all other Stream D briefs implement
against — first-mover sets the contract.

## Scope

Implement a `NotifyChannel` trait + `EmailChannel` impl that
sends approval-request emails (with HMAC-signed approve / reject
links) and receives operator replies via webhook.

## Files you may touch
- `crates/argo-channels/**` (NEW crate — this is the trait
  home)
- `Cargo.toml`
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
  (NEW — the inbound webhook route)
- `crates/argo-cli/src/handlers/gateway/mod.rs` (register the
  new route)
- `manifests/channels/email.toml` (config: postmark token,
  from-address, subject template)

## Files you must NOT touch
- Other Stream D channel impls (those briefs land separately)
- `Executor` trait
- `argo-mcp/**`

## Acceptance criteria
- [ ] `NotifyChannel` trait: `id() -> ChannelId`,
      `send(notification) -> Result<DeliveryReceipt>`,
      `poll() -> Result<Vec<InboundEvent>>` (no-op for email
      since inbound is webhook-driven, not poll-driven)
- [ ] `Notification` struct: target operator, action_id, risk
      level, prompt body, approve_url, reject_url, expiry
- [ ] `EmailChannel` impl wraps Postmark API
- [ ] Approve/reject URLs carry an HMAC-signed token
      (`approval_id` + `decision` + `exp`); the gateway route
      validates HMAC and emits an `Approval` event into the
      same approval queue the CLI uses
- [ ] Idempotent: if the same `approval_id` is already
      settled, the gateway returns "already resolved by X" with
      a 200; never double-settles
- [ ] Per-action manifest schema: `[action.X.approval]
      channels = ["email"]` works end-to-end
- [ ] Per-operator preference: a `quiet_hours` table in the
      operator config skips email send during off-hours (logs
      a deferred-send event instead)
- [ ] Integration test: send a fake notification (Postmark
      sandbox or a test SMTP), receive a fake webhook reply,
      assert the approval queue settles
- [ ] `cargo clippy -p argo-channels -- -D warnings` clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-D-email`
- PR title: `channels: NotifyChannel trait + email (Postmark) approval channel (D-email)`

## Notes
- Postmark > Sendgrid for setup speed; pick Postmark unless
  there's a strong reason.
- HMAC: use `hmac` + `sha2` crates; secret comes from env
  `ARGO_CHANNEL_HMAC_SECRET`.
- Inbound webhook URL: `POST /channels/email/webhook`. Postmark
  sends a JSON body; verify Postmark's signature header.
- The trait shape locked here is the contract for D-slack,
  D-sms, D-push, D-webinbox. Get it right.
