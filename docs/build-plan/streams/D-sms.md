# Stream D — SMS approval channel (Twilio)

**Owner**: codex. **Depends on**: D-email merged.

## Scope

`SmsApprovalChannel` impl. Sends an SMS with a short approve /
reject link (HMAC-signed); operator clicks → gateway
`/channels/sms/decision/<token>` route settles approval.
Reserved for high-risk actions (per manifest).

## Files you may touch
- `crates/argo-channels/src/lib.rs` — narrowly scoped: add ONE
  line `pub mod sms;` (or `mod sms;`). No other edits.
- `crates/argo-channels/src/sms.rs` (NEW file)
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
  (extend with `GET /channels/sms/decision/<token>`)
- `manifests/channels/sms.toml`

## Files you must NOT touch
- Other channel impls
- `NotifyChannel` trait

## Acceptance criteria
- [ ] `SmsApprovalChannel` impl of `NotifyChannel`
- [ ] Twilio API client; phone numbers via per-operator config
- [ ] Message body includes one approve link and one reject
      link, HMAC-signed
- [ ] Tappable from a phone (URL gets HTTPS-Only-Once-Open
      treatment — settle on first tap, idempotent if reopened)
- [ ] Twilio token + from-number via env
- [ ] Integration test gated on `TWILIO_TEST_*` envs
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-D-sms`
- PR title: `channels: SMS approval channel via Twilio (D-sms)`
