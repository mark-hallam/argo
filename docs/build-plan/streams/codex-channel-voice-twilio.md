# V1.4 — Voice approval channel (Twilio)

**Owner**: codex. **Depends on**: D-sms (#782) merged
(reuses Twilio creds).

## Scope

Voice approval channel: worker rings the operator, reads a
TTS message asking for approval, captures DTMF keypress
(1=approve, 2=deny, 9=request more details). Reuses existing
Twilio account from D-sms.

Less ergonomic than Slack/iMessage for approval but useful
for high-stakes actions (financial transactions, customer
contact) where the operator should physically pick up.

- New `VoiceApprovalChannel` in argo-channels.
- Outbound: Twilio Voice REST API → ringing → TwiML
  `<Gather>` for DTMF capture.
- Webhook handler at `/twilio/voice/decision` validates
  Twilio signature + emits approve/deny event.
- TTS via Anthropic or fallback to Twilio's built-in `<Say>`.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-channels/src/voice.rs` (NEW)
- `crates/argo-channels/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/src/handlers/gateway/routes_voice.rs` (NEW —
  webhook handler)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: pub mod
- `crates/argo-channels/tests/voice_smoke.rs` (NEW)

## Acceptance criteria
- [ ] Outbound call places ringing successfully (mocked
      Twilio response)
- [ ] DTMF 1 → approve, 2 → deny, 9 → request-details
- [ ] Webhook validates Twilio signature (rejects forged)
- [ ] TwiML `<Say>` template renders the action description
- [ ] HMAC approval token in DTMF callback URL prevents replay
- [ ] 5 unit tests + 1 integration test gated on
      `TWILIO_VOICE_FROM_NUMBER`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-channel-voice-twilio`
- PR title: `channels: voice approval via Twilio (codex-channel-voice-twilio)`
