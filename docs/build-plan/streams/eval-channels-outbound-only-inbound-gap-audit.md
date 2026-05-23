# eval-channels-outbound-only-inbound-gap-audit

**Owner**: claude or codex.
**Depends on**: nothing.
**Authored against**: origin/main `ca02058c9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-channels-outbound-only-inbound-gap-audit`.

🟡 **P2 — verify the 3 outbound-only channels (sms / push / voice) either have an inbound path or are documented as no-reply by design.** Source: PR #2111 §"inbound-coverage matrix (3 channels outbound-only + discord ed25519 gap as follow-ups)" + `mission-vs-code-gap-manifest.md` §91 (channel inbound paths).

`eval-channels-inbound-coverage` (MERGED in #2111) shipped a matrix of all 11 v1 channels showing which have inbound (approval-reply) wired vs only outbound. 3 channels are flagged as outbound-only:
- **sms** — outbound via twilio-equivalent; inbound (operator replies "YES" to approve) not wired.
- **push** — outbound via APNs/FCM; inbound is structurally inapplicable (no reply channel for push) but this is not documented.
- **voice** — outbound via TTS+TwilioCall-equivalent; inbound (DTMF reply, voicemail callback) not wired.

Mission claim 6 says "each channel impl handles BOTH outbound (request approval) AND (where applicable) inbound (receive the operator's reply)" — the **"where applicable"** clause covers `push` (no reply path), but `sms` and `voice` both have natural reply paths and the gap is real.

Plus a 4th finding from PR #2111: **discord ed25519 verification gap** — the inbound webhook for discord doesn't verify the ed25519 signature on the incoming interaction payload. This is a security gap, not an inbound-wiring gap.

This brief is the read-only audit that:
1. **Confirms** each of the 3 channels' outbound-only status (verify by reading the channel impl + handler).
2. **Decides** per-channel whether inbound is structurally applicable or genuinely missing.
3. **Files follow-up briefs** for any genuine missing inbound (e.g. `mission-sms-wire-inbound-twilio-webhook`, `mission-voice-wire-inbound-dtmf-callback`).
4. **Files a follow-up brief for the discord ed25519 verification gap** (`mission-discord-inbound-verify-ed25519-signature`).

The output is a single eval report + 3-5 follow-up brief stubs (one per finding).

## Scope

1. Read `crates/argo-channels/src/sms.rs`, `push.rs`, `voice.rs`. For each, document the outbound surface (`Channel::send` impl) + scan for any sibling `_inbound.rs` files. Confirm none exist (the gap-manifest claim is current).
2. Read `crates/argo-cli/src/handlers/routes_channels_discord.rs` (or wherever the discord inbound webhook lives). Locate the signature-verification step — confirm absence or weakness; cite line numbers.
3. For each of the 3 outbound-only channels, decide structural applicability:
   - **sms**: applicable (Twilio inbound webhook is a standard pattern). File `mission-sms-wire-inbound-twilio-webhook`.
   - **push**: structurally inapplicable (no reply channel exists for vendor APNs/FCM by design). Document this in the report; do NOT file a follow-up brief.
   - **voice**: applicable (DTMF or voicemail-callback). File `mission-voice-wire-inbound-dtmf-callback`.
4. File `mission-discord-inbound-verify-ed25519-signature` (P1 — security gap, ed25519 verification is required by Discord's published spec).
5. Render report at `docs/skycot/evals/scheduled/2026-05-23/eval-channels-outbound-only-inbound-gap-audit.md` with: per-channel verdict, file-line cites, follow-up brief filenames.

## Files you may touch

- `docs/skycot/evals/scheduled/2026-05-23/eval-channels-outbound-only-inbound-gap-audit.md` — NEW report.
- `docs/build-plan/streams/mission-sms-wire-inbound-twilio-webhook.md` — NEW follow-up.
- `docs/build-plan/streams/mission-voice-wire-inbound-dtmf-callback.md` — NEW follow-up.
- `docs/build-plan/streams/mission-discord-inbound-verify-ed25519-signature.md` — NEW follow-up.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — update §91 with this audit's verdict.

## Files you must NOT touch

- Any `crates/argo-channels/src/` Rust file — this is a read-only audit.
- Any `crates/argo-cli/src/handlers/routes_channels_*.rs` — read-only.
- Test files — no new tests in this brief; the follow-up briefs ship the tests.

## Acceptance criteria

- [ ] Eval report exists with verdict for each of the 3 outbound-only channels.
- [ ] Eval report cites the exact file:line for each channel's outbound surface + the absence of the inbound sibling.
- [ ] At least 2 follow-up briefs filed (sms + voice; push correctly stays unfiled).
- [ ] Discord ed25519 follow-up brief filed.
- [ ] Gap-manifest §91 row updated with audit verdict.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- No automated tests (audit-only).
- The follow-up briefs (sms inbound, voice inbound, discord verify) carry their own test plans.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Wiring the inbound paths themselves — owned by the 3 follow-up briefs.
- Auditing WhatsApp / Teams / Matrix inbound — those are DEFERRED-v1.x at the outbound layer per operator decision, so inbound is moot until they're un-deferred.
- Auditing email + slack inbound — those are already wired (`email_inbound.rs`, `slack_inbound.rs`) and not part of this gap.
- Authoring tests for the existing outbound channels — separate sweeps.

~200-300 LOC of markdown across 1 eval report + 3 brief stubs. No Rust touched.
