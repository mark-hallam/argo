# mission-voice-wire-inbound-dtmf-callback

**Owner**: claude or codex.
**Depends on**: none.
**Authored against**: origin/main `adc92d978` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-voice-wire-inbound-dtmf-callback`.

P1 claim-6 follow-up. The brief id is retained from the parent audit,
but current main already has the DTMF callback route. Do not add a
second route. The remaining gap is audit and settlement parity.

## Scope

Bring the existing Twilio Voice DTMF route up to the same approval-reply
contract as the other channel routes:

1. Keep `POST /twilio/voice/decision` as the canonical route.
2. Preserve Twilio `X-Twilio-Signature` verification and the signed
   voice callback token.
3. Ensure successful approve / deny / request-details replies emit an
   `AuditKind::ApprovalReplied` row with channel `voice`, replier,
   reply kind, reply body or digit, and timestamp.
4. Preserve idempotency for late replies.
5. Add tests proving audit emission and no duplicate audit row on a
   second reply.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_channels_voice.rs`
- `crates/argo-cli/src/handlers/gateway/routes_channels_voice_tests.rs`
- Adjacent gateway test helpers if existing helpers already cover
  audit-row setup for approval replies.

## Acceptance

- [ ] No duplicate voice route is added.
- [ ] Valid DTMF approve and deny callbacks emit one `approval.replied`
      audit row.
- [ ] Duplicate callbacks remain idempotent and do not emit a second
      `approval.replied` row.
- [ ] Existing Twilio signature and callback-token rejection tests still
      pass.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.

## Notes

Current evidence: `routes_channels_voice.rs:24` recognizes the route,
`routes_channels_voice.rs:34` requires Twilio signature, and
`routes_channels_voice.rs:49` verifies the voice callback token. The
file does not import `AuditKind` or `AuditSink`, while the shared
settlement helper emits `ApprovalReplied` in `routes_channels.rs:100`.
