# V1.4 — WhatsApp approval channel via Twilio

**Owner**: codex. **Depends on**: D-sms (#782) merged
(reuses Twilio creds + signature validation pattern).

## Scope

WhatsApp Business API exposed via Twilio is structurally
identical to D-sms (Twilio SMS) except:
- Different `From` number (whatsapp:+14155238886 vs SMS)
- Body can include richer formatting (limited markdown)
- Inbound webhook arrives at a different path

This brief extends the existing SMS infrastructure rather
than building from scratch.

- New `WhatsAppApprovalChannel` in argo-channels (parallel to
  `SmsApprovalChannel`)
- Reuses existing Twilio HTTP client + HMAC validation
- Webhook handler at `/twilio/whatsapp/decision`

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-channels/src/whatsapp.rs` (NEW)
- `crates/argo-channels/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/src/handlers/gateway/routes_whatsapp.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: pub mod
- `crates/argo-channels/tests/whatsapp_smoke.rs` (NEW)

## Acceptance criteria
- [ ] Channel sends WhatsApp message + receives reply via webhook
- [ ] HMAC signature validation (rejects forged)
- [ ] Approve/deny keywords parsed (yes/y/✓ vs no/n/✗) from
      free-form reply
- [ ] HMAC token in approval message prevents replay
- [ ] 5 unit tests + 1 integration test gated on
      `TWILIO_WHATSAPP_FROM_NUMBER`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-channel-whatsapp-twilio`
- PR title: `channels: WhatsApp approval via Twilio (codex-channel-whatsapp-twilio)`
