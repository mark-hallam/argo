# codex-channel-dispatcher-voice-completion — Wire voice channel into EnvChannelSender; defer iMessage/WhatsApp/Teams/Matrix to v1.x (F-07 P1)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-channel-dispatcher-voice-completion`.
**Source**: F-07 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **Claim 6 (multi-channel approvals):** Mission names "email, Slack, SMS, push, voice, and web" (`docs/skycot/MISSION.md:45`). Voice is named explicitly — must be reachable for v1.
- iMessage / WhatsApp / Teams / Matrix are NOT in Claim 6's 6-channel set. Best-practice for v1: explicitly defer to v1.x rather than ship unreachable code.

## Scope

P1 wiring + scope honesty. `EnvChannelSender` (`crates/argo-cli/src/handlers/worker_actions_notify.rs:309-319`) supports 8 kinds: `push`, `webinbox`, `slack`, `email`, `sms`, `discord`, `chat`, `sse`. **Voice is missing** despite `crates/argo-channels/src/voice.rs:154` having a working implementation. Same for iMessage/WhatsApp/Teams/Matrix — implementations exist but the auto-approval ask dispatcher routes them to the "unsupported channel" error path (`worker_actions_notify.rs:319`).

This is precisely the drift pattern of "infrastructure shipped without runtime wiring."

Operator decision (locked in this brief based on MISSION.md): wire **voice** (named in Claim 6) for v1. Explicitly DEFER iMessage/WhatsApp/Teams/Matrix to v1.x in V1_DEFINITION.md + docs, so the codebase claim matches reality.

Scope:
- Add `"voice" => notify_voice::send_voice(ref_, n).await` arm to `EnvChannelSender::send_via` (`worker_actions_notify.rs:309`).
- Confirm `crates/argo-channels/src/voice.rs` has the right outbound API (check `:154` for the send entrypoint).
- Add a `notify_voice` re-export module if it doesn't exist (mirror `notify_slack` / `notify_email` / `notify_sms` patterns).
- Add config support: `[channels.voice]` block with Twilio (or Plivo, or operator-chosen provider) credentials.
- Add outbound + inbound round-trip test for voice (mocked provider HTTP).
- Update `docs/skycot/V1_DEFINITION.md` to declare: voice = v1; iMessage/WhatsApp/Teams/Matrix = DEFERRED-v1.x.
- Update `MISSION_BACKLOG.md` lines 285, 286, 298, 300 (per audit) — flip iMessage/voice/Teams/Matrix rows to reflect the v1 vs v1.x split.

LOC: ~300.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_notify.rs` — add voice arm; update error message to list deferred-channel names explicitly. ≤30 modified lines.
- `crates/argo-cli/src/handlers/notify_voice.rs` (NEW) — thin module wrapping `argo-channels::voice::send`. ≤80 LOC.
- `crates/argo-channels/src/voice.rs` — narrowly: ensure `send_voice(ref, n)` is the public outbound entrypoint at `:154` or expose it. ≤30 modified lines.
- `crates/argo-config/src/sections.rs` — `[channels.voice]` config block (provider, account_sid, auth_token_env, from_number). ≤30 modified lines.
- `crates/argo-cli/src/handlers/notify_voice_tests.rs` (NEW) — mock-provider outbound test. ≤80 LOC.
- `crates/argo-cli/tests/voice_approval_roundtrip.rs` (NEW) — integration test for voice approval round-trip (mocked Twilio webhook). ≤120 LOC.
- `docs/skycot/V1_DEFINITION.md` — narrowly: claim 6 channel status table (v1 vs v1.x split). ≤20 modified lines.
- `docs/build-plan/MISSION_BACKLOG.md` — lines 285/286/298/300 status updates. ≤10 modified lines.

## Files you must NOT touch

- `argo-channels` per-channel implementations (slack, email, sms, push, webinbox) — they work; don't refactor.
- iMessage / WhatsApp / Teams / Matrix implementations — deferred per operator decision; leave as-is (don't remove, but don't wire either).
- Inbound webhook routes for OTHER channels — voice may have its own webhook landing for approval replies; iMessage etc. stay deferred.

## Acceptance criteria

- [ ] `EnvChannelSender::send_via` handles `"voice"` kind without falling through to the error path.
- [ ] `[channels.voice]` config block parses; credentials loaded from env per the section's contract.
- [ ] Outbound test: voice notification triggers mock provider HTTP with correct payload.
- [ ] Inbound test: provider webhook → approval reply propagates through `inbound_route_voice` (or equivalent).
- [ ] V1_DEFINITION.md Claim 6 channel status table reflects: v1 = email/Slack/SMS/push/voice/web; DEFERRED-v1.x = iMessage/WhatsApp/Teams/Matrix.
- [ ] Error message for unsupported channel lists the deferred set so operators know what's not v1.
- [ ] `cargo test -p argo-cli -p argo-channels voice` passes.
- [ ] `cargo clippy -p argo-cli -p argo-channels -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-channel-dispatcher-voice-completion`
- PR title: `feat(channels): wire voice into auto-approval; defer iMessage/WhatsApp/Teams/Matrix to v1.x (codex-channel-dispatcher-voice-completion) — F-07 P1`

## Notes for the implementer

- The voice provider choice (Twilio vs Plivo vs other) is open per operator. Use Twilio as default unless operator specifies; the brief can ship with a TwilioVoiceClient and Plivo can be DEFERRED.
- Inbound webhook signature verification is required for voice provider webhooks (parity with Slack signing-secret pattern in `routes_channels_slack.rs:97`).
- After this brief, V1_DEFINITION.md will honestly claim only the channels the codebase can actually dispatch through.
