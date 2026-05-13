# Voice channels — what ships in v1.0

**Audience:** operators and brief authors who see "voice channel
exists" and need to know whether bidirectional voice CHAT is part
of v1.0.

**Short answer:** No. The voice surface that ships in v1.0 is an
APPROVAL channel — a one-way Twilio call that reads an approval
request aloud and reads back the operator's DTMF (touch-tone)
choice. There is no real-time conversational voice mode.

## What works today (v1.0)

**Voice as APPROVAL channel.** Backed by
`argo-channels::voice::VoiceApprovalChannel`
(`crates/argo-channels/src/voice.rs`).

Flow:

1. The runtime emits a high-risk approval request.
2. The voice channel places an outbound Twilio call to the
   configured operator phone number, using a TwiML body rendered
   by `render_voice_twiml`.
3. Twilio reads the approval prompt aloud via TTS.
4. The operator dials a key (DTMF). Twilio POSTs the digit back to
   the gateway with an HMAC-signed callback token
   (`sign_voice_callback_token` / `verify_voice_callback_token`).
5. The gateway resolves the approval as Approved or Denied.

Operationally:

- Voice is **reserved for `RiskLevel::High`**. Lower-risk
  notifications are deferred with reason
  `voice_reserved_for_high_risk` rather than placing a call.
- Per-channel rate-limit defaults to **1/min, 5/hour**
  (`RateLimitConfig::voice_default()`).
- Quiet-hours config is honored — if set and the current time
  falls inside the window, the send is deferred rather than
  placing a call.
- Required env: `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`,
  `TWILIO_VOICE_FROM_NUMBER` (or `TWILIO_FROM_NUMBER`),
  `ARGO_TWILIO_VOICE_DECISION_URL`, `ARGO_CHANNEL_HMAC_SECRET`.
- Test harness: see `crates/argo-channels/tests/voice_smoke.rs`.

## What does NOT work today

**Bidirectional voice CHAT** (full conversational mode where an
operator speaks naturally and the agent replies in voice).

Specifically, none of the following exist in v1.0:

- Speech-to-text (STT) ingest of operator audio.
- Real-time streaming/chunked protocol between Twilio and the
  runtime.
- Text-to-speech (TTS) pipe-back of agent replies as audio.
- WebRTC or SIP-trunked media handling.
- Any voice-side primitive beyond the HMAC-signed DTMF callback.

If you see a brief or vertical manifest that assumes voice
conversation, treat it as **premise drift**. The only voice
primitive shipped is the approval-by-DTMF flow above.

## What it would take to ship bidirectional voice chat

Out of scope for v1.0. A future build would need, at minimum:

- An STT provider (Whisper, Deepgram, or equivalent) wired to
  Twilio Media Streams or a SIP trunk.
- A chunked streaming protocol so audio flows in/out of the
  runtime turn-by-turn without buffering whole utterances.
- A TTS pipeline for agent replies (provider TBD; Twilio TTS used
  for approvals is too low-fidelity for sustained conversation).
- Media-plane infrastructure: WebRTC peering or SIP trunking with
  a media server (e.g. LiveKit, Jambonz).
- Latency budgets, barge-in handling, voice-VAD, and
  audit-trail capture for the audio stream.

If/when that work is scoped, the reserved brief slug is
`mission-voice-chat-bidirectional`. It does **not** exist today
— naming it here so future briefs can use the slot without
collision.

## Why this doc exists

Several vertical briefs across the swarm have referenced the
"voice channel" as if it were a conversation primitive. The
channel crate ships exactly one voice primitive (DTMF approval),
and disambiguating that from "voice chat" requires reading the
crate source. This page lets brief authors and reviewers cite
the v1.0 reality without spelunking.
