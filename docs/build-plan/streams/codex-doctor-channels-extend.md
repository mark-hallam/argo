# V1.4 — Doctor channels extend

**Owner**: codex. **Depends on**: H-channel-health-checks
(#828) merged + #890 (imessage) + #896 (voice) merged.

## Scope

`H-channel-health-checks` (#828) added `argo doctor channels`
checks for Postmark/Slack/Twilio/Expo. Since then, 2 new
channels landed:
- iMessage via Blue Bubbles (#890)
- Voice approval via Twilio (#896)

Both need health-check arms in `doctor_channels.rs`. Pattern is
mechanical:
- New `check_imessage()` — try GET /api/v1/server/info on the
  Blue Bubbles URL; report OK / FAIL / MISSING (manifest
  config absent)
- New `check_voice()` — try Twilio API ping (existing pattern
  used by SMS check); just verify TWILIO_VOICE_FROM_NUMBER
  manifest field is set + Twilio creds work

5s per-check timeout (consistent with existing checks).

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/doctor_channels.rs` —
  narrowly: 2 new check functions + 2 new dispatch arms
- `crates/argo-cli/src/handlers/doctor_channels_tests.rs`
  (NEW or extend if exists) — 4 unit tests (2 per channel:
  OK + MISSING)

## Acceptance criteria
- [ ] `argo doctor channels` reports imessage + voice checks
- [ ] OK status when manifest configured + endpoint reachable
- [ ] MISSING status when manifest absent
- [ ] FAIL status when endpoint reachable but auth fails
- [ ] 5s timeout per check
- [ ] 4 unit tests + extends existing smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-doctor-channels-extend`
- PR title: `doctor: extend channels check with imessage + voice (codex-doctor-channels-extend)`
