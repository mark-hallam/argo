# Mission load-bearing wire — Document voice_chat paper-only status

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `392baa27` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #277. `argo-channels::voice` exists as a Twilio-backed approval channel (verified: `crates/argo-channels/src/voice.rs:1` "Twilio-backed voice approval channel"), but **bidirectional voice CHAT** (operator + agent natural-language conversation over voice) was never shipped end-to-end. Operators have asked "can my worker take voice calls?" — answer is no in v1. Capture that explicitly so the swarm doesn't author premise-drifted briefs against a non-existent surface. **Tier-Doc.**

## Scope

Single-purpose doc-only brief:

1. **NEW** `docs/skycot/operator/voice-channels-status.md` — operator-facing reference:
   - **What works today** (v1.0): voice as APPROVAL channel — Twilio-backed inbound call where the operator listens to a TTS-rendered approval request and dials a key to approve/deny. Backed by `argo-channels::voice::VoiceApprovalChannel`. See `voice_smoke.rs` for the test harness.
   - **What does NOT work today**: bidirectional voice chat (full conversational mode). No STT pipeline, no real-time streaming, no voice-output of agent replies. The HMAC-signed callback token + DTMF flow is the *only* voice-side primitive.
   - **What it would take to ship**: STT (Whisper/Deepgram) + chunked stream protocol + TTS pipe-back + WebRTC or SIP-trunked media. Out of scope for v1.0; if/when relevant, the brief slug is `mission-voice-chat-bidirectional` (does NOT exist today).
   - **Why doc not code**: many vertical briefs reference "voice channel" expecting bidirectional chat. This doc lets the brief author + reviewer disambiguate without spelunking the channels crate.

2. **EDIT** `docs/skycot/RUNBOOK.md` — append a one-line cross-link under existing voice section: "Bidirectional voice chat: not shipped in v1.0 — see `voice-channels-status.md`."

3. **EDIT** `verticals/_starter-library/README.md` (if it lists channels) — annotate `voice` as "approval-only" so vertical authors don't assume chat semantics.

NOT in scope: writing the bidirectional voice chat code; adding STT/TTS deps; benchmarking voice latency.

## Files you may touch

- `docs/skycot/operator/voice-channels-status.md` (NEW, ≤ 200 LOC)
- `docs/skycot/RUNBOOK.md` — append ≤ 5 LOC
- `verticals/_starter-library/README.md` IF the file exists AND mentions voice; otherwise skip

## Files you must NOT touch

- `crates/argo-channels/` — read-only inputs
- Any code path or test
- Any vertical manifest

## Acceptance criteria

- [ ] `voice-channels-status.md` exists with the 4 sections above
- [ ] Worded for an operator who's never read the crate source
- [ ] RUNBOOK cross-link added
- [ ] `verticals/_starter-library/README.md` touched only if relevant (worker judges)
- [ ] Zero code touches (verify `git diff --stat -- crates/` is empty)
- [ ] BUILD_STATUS.md row updated

## Branch + PR

- Branch: `claude-mission-document-voice-chat-paper-only`
- PR title: `docs: voice_chat paper-only status (mission-document-voice-chat-paper-only)`

## Notes

- Keep the doc operator-facing. The brief author who wrote MISSION_BACKLOG #277 was thinking from the perspective of an operator who got confused by "voice channel exists" claims. Aim the writing at that audience.
- The brief slug `mission-voice-chat-bidirectional` reserved in the body is NOT being authored here — just naming it so future briefs can use the slot without colliding.
- Worker may notice tiny additional cleanups (e.g. `voice_smoke.rs` doc comment that says "voice chat" instead of "voice approval") — leave them for a separate brief; this is doc-only.
