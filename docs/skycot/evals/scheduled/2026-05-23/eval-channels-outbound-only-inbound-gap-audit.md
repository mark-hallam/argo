# eval-channels-outbound-only-inbound-gap-audit

**Authored**: 2026-05-24 by codex worker.
**Audited against**: origin/main `960750d49b` on 2026-05-24.
**Brief**: `docs/build-plan/streams/eval-channels-outbound-only-inbound-gap-audit.md`.
**Gap-manifest row touched**: `Channel inbound paths`.

## Verdict

This brief was stale in a useful way. The original hypothesis said
`sms`, `push`, and `voice` were outbound-only. Current main shows:

| channel | outbound surface | inbound surface | verdict | follow-up |
|---|---|---|---|---|
| `sms` | `crates/argo-channels/src/sms.rs:127` sends Twilio Messages API requests; `poll()` returns empty at `sms.rs:196`. | No `routes_channels_sms.rs`, no `sms_inbound.rs`; only tests mention `/channels/sms/decision/...`. | Real gap: outbound-only despite a natural Twilio inbound SMS reply path. | `mission-sms-wire-inbound-twilio-webhook` |
| `push` | `crates/argo-channels/src/push.rs:147` sends Expo push payloads; payloads carry `/channels/push/decision/<token>` at `push.rs:122`. | `crates/argo-cli/src/handlers/gateway/routes_channels_push.rs:15` recognizes device and decision routes, and `push_response` verifies the signed token at `routes_channels_push.rs:46`. | No follow-up. Push is not a provider-reply channel; the shipped deep-link decision route is the applicable inbound path. | none |
| `voice` | `crates/argo-channels/src/voice.rs:159` sends Twilio Voice calls; `twiml_for` embeds a signed callback URL at `voice.rs:135`. | `crates/argo-cli/src/handlers/gateway/routes_channels_voice.rs:24` recognizes `/twilio/voice/decision`, verifies Twilio signature at `routes_channels_voice.rs:34`, verifies the callback token at `routes_channels_voice.rs:49`, and maps DTMF digits at `routes_channels_voice.rs:56`. | Route exists, so the old "wire DTMF callback" premise is stale. The remaining claim-6 gap is parity: this path settles directly and does not emit the shared `ApprovalReplied` audit row. | `mission-voice-wire-inbound-dtmf-callback` (stale id retained; content scopes audit parity) |
| `discord` | `crates/argo-channels/src/discord.rs:102` sends Discord webhook messages. | `crates/argo-cli/src/handlers/gateway/routes_channels_discord.rs:17` accepts Discord decisions and verifies only the embedded approval token at `routes_channels_discord.rs:30`. | Security gap: the route does not read or verify `X-Signature-Ed25519` / `X-Signature-Timestamp`; repo grep finds no Discord Ed25519 verification in this route. | Existing TODO `mission-channel-discord-ed25519-verify` |

The gateway dispatcher confirms these routed inbound paths: it includes
Discord, push, and voice predicates in `routes_channels.rs:45`,
`routes_channels.rs:47`, and `routes_channels.rs:50`, then dispatches
to their handlers at `routes_channels.rs:59`, `routes_channels.rs:65`,
and `routes_channels.rs:74`.

## Notes

- `push` should stay documented as "inbound via app/browser deep link",
  not "no reply by design". The approval token in the deep link is the
  reply mechanism.
- `voice` should not get a duplicate route. The follow-up brief keeps
  the parent-requested filename for traceability, but explicitly scopes
  the work to audit parity on the existing DTMF route.
- `sms` is the only mission-named channel in this slice with no inbound
  route on current main.
- `teams` and `matrix` remain broader outbound/inbound gaps, but they
  are outside this brief's `sms` / `push` / `voice` focus and outside
  the v1.0 channel promise.

## Acceptance

- Eval report exists with verdict for each of the 3 outbound-only
  candidates: done.
- Exact file-line citations included for outbound and inbound evidence:
  done.
- Follow-up coverage filed: new stubs
  `mission-sms-wire-inbound-twilio-webhook` and
  `mission-voice-wire-inbound-dtmf-callback`, plus existing TODO
  `mission-channel-discord-ed25519-verify`. No duplicate Discord
  brief was authored.

## Sensible deviations

- The parent brief requested a new
  `mission-discord-inbound-verify-ed25519-signature` stub, but current
  main already has `mission-channel-discord-ed25519-verify` as a TODO
  row and brief. This audit cites that existing row instead of creating
  a parallel duplicate.
- The parent brief requested a voice inbound wiring follow-up. Current
  main already has the DTMF route, so the retained voice follow-up
  scopes audit parity rather than duplicate route wiring.
- Gap-manifest `Channel inbound paths` row updated with this audit's
  corrected verdict.
