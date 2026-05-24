# mission-sms-wire-inbound-twilio-webhook

**Owner**: claude or codex.
**Depends on**: none.
**Authored against**: origin/main `adc92d978` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sms-wire-inbound-twilio-webhook`.

P1 claim-6 follow-up. SMS can send approval requests through Twilio
Messages, but there is no gateway route for Twilio inbound SMS replies.

## Scope

Add a Twilio inbound SMS approval route:

1. Add `crates/argo-cli/src/handlers/gateway/routes_channels_sms.rs`.
2. Recognize `POST /twilio/sms/decision` and/or
   `POST /channels/sms/decision`.
3. Verify `X-Twilio-Signature` using the same Twilio signature helper
   shape as the WhatsApp and voice routes.
4. Parse `From` and `Body`; accept explicit signed approval tokens and
   common replies such as `YES` / `APPROVE` and `NO` / `REJECT` when
   they can be bound to an approval token or id.
5. Delegate settlement through the shared channel `settle` helper so
   `AuditKind::ApprovalReplied` is emitted exactly once.
6. Wire the route into `routes_channels.rs` and `gateway/mod.rs`.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
- `crates/argo-cli/src/handlers/gateway/routes_channels_sms.rs`
- `crates/argo-cli/src/handlers/gateway/mod.rs`
- `crates/argo-cli/src/handlers/gateway/routes_channels_sms_tests.rs`

## Acceptance

- [ ] Twilio signature is required for SMS inbound requests.
- [ ] Valid approve and reject SMS replies settle the pending approval.
- [ ] Late duplicate replies are idempotent and do not emit duplicate
      `approval.replied` audit rows.
- [ ] Missing or invalid signatures return unauthorized.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Focused gateway tests for the new route pass.

## Notes

Current evidence: `crates/argo-channels/src/sms.rs:127` sends SMS, and
there is no `routes_channels_sms.rs` or `sms_inbound.rs` on current
main.
