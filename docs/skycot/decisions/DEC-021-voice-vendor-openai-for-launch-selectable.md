---
id: DEC-021
date: 2026-05-30
status: active
supersedes: none
---

# DEC-021 — Voice vendor = OpenAI for launch; vendor-selectable + user-selectable

## Decision

The live-voice vendor for launch is **OpenAI**, wired as **discrete REST calls**
behind the existing `argo-voice-io` trait layer ([[DEC-019]] build-out, voice
first):

- **STT** (speech-to-text) via `POST /v1/audio/transcriptions` (whisper-1,
  multipart upload).
- **TTS** (text-to-speech) via `POST /v1/audio/speech` (tts-1, `voice` param).

The true **OpenAI Realtime** streaming socket (lower-latency, full-duplex) is a
**later latency upgrade**, not launch scope — the discrete STT+TTS path gives a
working, truth-bounded voice loop now without the heavier Realtime integration.

The provider is **vendor-selectable AND user-selectable**:

- Vendor selection via the `ARGO_VOICE_PROVIDER` env var (default mock until a
  key is present; `openai` once `OPENAI_API_KEY` is configured).
- User selection via a `voice` group in the `workspace_settings` table (the same
  generic settings store shipped in [[DEC-019]] build-out item T1, migration
  v85), so a workspace can pick its voice provider.

## Why

- Operator call (2026-05-30): OpenAI for launch, but the architecture must let us
  swap vendors and let users choose. Single vendor = launch simplicity; the trait
  + env + workspace-settings seams keep us un-locked-in.
- The discrete STT/TTS path reuses the real truth-boundary + audit primitives
  unchanged (the voice loop wraps the already-shipped `pitch-ask` logic), so
  voice inherits the moat with zero duplication.
- OpenAI's `whisper-1` + `tts-1` REST endpoints are stable, key-simple, and the
  MCP voice server's existing OpenAI backend de-risks the wiring.

## How it's built

- New adapter in `crates/argo-cli/src/handlers/gateway/routes_voice_io/` (e.g.
  `openai.rs`) implementing `SpeechToText` (transcriptions) + `TextToSpeech`
  (speech), with a `Provider::OpenAi` arm in `provider.rs`.
- Key sourced via `argo_config::cred::get_unscoped("OPENAI_API_KEY")`.
- Tests are **key-gated** (skip when no key present) so CI/cold builds stay green
  without a secret.
- The LIVE check needs `ARGO_VOICE_PROVIDER=openai` + `OPENAI_API_KEY` in
  `~/.argo/env.local`; build + unit-test do not.

## Status note (honesty)

Voice L1 (trait layer + mocks, commit `4e891c0b6`) and L2 (gateway routes +
truth-bounded pitch voice-turn, commit `5a98d63dc`) are shipped + tested with the
**mock** provider. The **real OpenAI adapter is 🚧 in-progress this sprint**
(was BLOCKED-on-vendor-decision; this ADR unblocks it). Browser push-to-talk
(L3) remains planned.

Related: [[DEC-019]] (build-out, voice first), [[DEC-022]] (Sprint-2 includes
real voice), [[DEC-001]] (enterprise focus — vendor flexibility matters to
buyers).
