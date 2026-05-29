# Design plan — Live voice for AI teammates (Portia pitching out loud)

**Status:** ready-for-go implementation plan. Authored 2026-05-29 (overnight).
**Closes:** competitive gap #4 (live voice) + the operator's stated keenest
interest — *"Portia should be able to pitch to an investor live or on a
Teams/Zoom call."*
**Effort:** L1 ≈ 1–2 wk · L2 ≈ 2–3 wk · L3a ≈ 1 wk · L3b ≈ 3–5 wk (phased).

---

## Plain-English summary (for the operator)

Today our agents can *write* (Portia drafts a pitch, Grant drafts a follow-up).
They cannot *speak or listen* in a live conversation. The "voice plumbing"
(`argo-voice-io`) is a placeholder with nothing real plugged in — no
speech-to-text (turning what someone says into text), no text-to-speech
(turning the agent's words into a spoken voice), and no way to join a
Teams/Zoom call.

This plan adds real voice in four steps, smallest-first, so each step is
demoable on its own:

- **L1 — Give the agents a real voice + ears.** Plug a real speech vendor
  (e.g. ElevenLabs for the voice, Deepgram/OpenAI for the listening) behind the
  existing placeholder. After L1: we can turn any text into Portia's spoken
  voice and turn a recording into text. *(The single decision you need to make:
  which vendor — see below.)*
- **L2 — A real back-and-forth conversation.** Wire ears → Portia's existing
  truth-bounded brain → voice, in a loop, so you can have a spoken conversation
  with her. After L2: Portia pitches out loud and answers spoken questions,
  still refusing to over-claim (the truth boundary works the same whether she's
  typing or talking).
- **L3a — Talk to her in the browser.** A "press to talk" button on the
  `/pitch` page. After L3a: the investor demo can be done live, in the browser,
  with your voice — no phone, no call platform.
- **L3b — Join a Teams/Zoom/Meet call.** A meeting bot that dials into a real
  call as a participant. This is the big one (each platform is its own
  integration) and is genuinely a separate, larger project.

**Recommendation:** do **L1 + L2 + L3a** as one ~4–6 week push — that gets you
"Portia pitches live in the browser," which is 90% of the wow with 20% of the
effort. Treat **L3b (meeting-join)** as a separate funded project once the
browser version proves the concept.

---

## What exists today (verified against the repo)

- `crates/argo-voice-io` — registers an **in-memory stub** only. No real STT/TTS
  in the agent path. (It is currently an *orphan* crate — nothing in the live
  runtime path constructs it.)
- Twilio **DTMF** (keypad-tone) handling exists for approvals — this is *not*
  conversational voice and should not be conflated with it.
- **The hard part is already done:** Portia's brain — `POST /api/v1/pitch/ask`
  — is real (LLM via `claude-code-cli`), truth-bounded
  (`argo_capability::evaluate_claim` → Allow/Escalate/Block), audited, and
  approval-gated. Voice is "just" an input/output skin around this proven core.
- The MCP voice connector already references `openai`/`elevenlabs` backends —
  a seed for L1's adapter, not a finished implementation.

## The one decision the operator must make (L1 vendor)

| Option | Voice (TTS) | Listening (STT) | Why pick it |
|---|---|---|---|
| **A — ElevenLabs + Deepgram** (recommended) | ElevenLabs (best-in-class natural voice, voice cloning) | Deepgram (fast, accurate, cheap streaming) | Best demo wow; ElevenLabs voice is the one investors will remember |
| **B — OpenAI Realtime** | OpenAI | OpenAI | One vendor, one key, lowest integration effort; tightly-coupled turn-taking |
| **C — Azure Speech** | Azure | Azure | If enterprise buyers demand Microsoft-stack + data-residency |

**Each option needs an API key the agent doesn't currently hold** — this is the
one external dependency that blocks starting L1. Recommendation: **A** for the
demo; keep the adapter vendor-agnostic so B/C swap in by config (consistent with
our "provider manifests, not adapters" invariant).

## Architecture — keep voice vendor-agnostic behind a trait

Mirror the provider-manifest pattern (TOML config, not hardcoded adapters):

```
argo-voice-io (un-orphan it)
├── trait SpeechToText   { async fn transcribe(&self, audio) -> Transcript }
├── trait TextToSpeech   { async fn synthesize(&self, text, voice) -> AudioStream }
├── adapters/elevenlabs.rs   (TTS)   — feature-gated, key from env/manifest
├── adapters/deepgram.rs     (STT)   — feature-gated
├── adapters/openai_realtime.rs      — both, single-socket
└── VoiceManifest (TOML): which adapter, which voice id, latency budget
```

Voice id is a **manifest parameter** (per the existing "voice-as-manifest" note):
Portia's voice ≠ Grant's voice ≠ a customer's cloned brand voice.

## Implementation phases (each independently shippable + demoable)

### L1 — real STT/TTS behind the trait (~1–2 wk)
- Un-orphan `argo-voice-io`: define the two traits + one TTS + one STT adapter.
- `POST /api/v1/voice/tts {text, voice_id}` → audio bytes; `POST
  /api/v1/voice/stt {audio}` → transcript. (Additive gateway routes; mirror the
  hand-rolled dispatcher pattern in `routes_*`.)
- Key handling via `~/.argo/env.local` (like `ARGO_PITCH_LLM_PROVIDER`); never
  in code, never in URLs (privacy rule).
- **Verify:** `argo voice say "..."` CLI writes a wav you can play; round-trip a
  known clip through STT and assert the transcript.

### L2 — agent voice-loop (~2–3 wk)
- New `voice_turn` orchestration: STT(audio) → **the existing
  `pitch-ask` path** (LLM + truth boundary + audit) → TTS(answer) → audio.
- Barge-in / turn-taking handling (stop speaking when interrupted).
- The truth boundary is unchanged — a Blocked claim becomes a spoken "I can't
  confirm that — let me bring in a human," same as the text path.
- **Verify:** scripted audio question in → correct spoken answer out; a
  known over-claim question → spoken deferral + an audit row + approval-queue
  entry (proves the boundary holds in voice).

### L3a — browser push-to-talk on /pitch (~1 wk)
- `MediaRecorder` capture in the `/pitch` page → stream to L2 → play the
  returned audio. A waveform + live transcript for the demo.
- **Verify:** click-to-talk in the browser, hear Portia answer, see the
  transcript + the truth-boundary banner when she defers.

### L3b — meeting-join bot (~3–5 wk, separate project)
- A headless participant that joins Teams/Zoom/Meet (each is its own SDK/bot
  framework: Zoom Meeting SDK, Microsoft Graph + Teams bot, Meet via
  `meet-add-on`/recall.ai-style). Audio in → L2 → audio out into the call.
- This is the genuine "AI joins your call" capability and warrants its own
  brief tree. Recommend a single-platform spike (Zoom) first.

## Risks / honesty notes
- **External keys are the only true blocker to starting** — flag this to the
  operator (it's why L1 hasn't begun).
- Latency is the make-or-break for "feels live" — budget < 800ms round-trip;
  ElevenLabs Conversational AI 2.0 and OpenAI Realtime are both designed for
  this, but headline-latency vendor claims are unbenchmarked (see gap analysis).
- NO THEATER: until L1 lands, do **not** show a fake voice demo. The text pitch
  is real and impressive on its own.

## Brief decomposition (ready to author into the swarm)
`voice-L1-stt-tts-adapters` → `voice-L1-gateway-routes` →
`voice-L2-voice-loop-over-pitch-ask` → `voice-L2-truth-boundary-spoken-deferral`
→ `voice-L3a-browser-push-to-talk` → (separate) `voice-L3b-zoom-meeting-bot-spike`.
Each carries the freshness header + names its consumer per DRIFT_PREVENTION.md.
