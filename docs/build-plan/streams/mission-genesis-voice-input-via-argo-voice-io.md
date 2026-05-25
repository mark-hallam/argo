# mission-genesis-voice-input-via-argo-voice-io

**Owner**: claude.
**Authored against**: origin/main `a85963b34` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-voice-input-via-argo-voice-io`.
**Depends on**: `mission-genesis-gateway-route-streaming-api` + `mission-genesis-spa-page-describe-and-watch` MERGED.

🔴 **P0 MISSION CLAIM 3 — Phase 2 Onboarding Studio — voice-driven role description.**

Per memory [[genesis-is-ai-teammate-onboarding-studio]] (operator-clarified 2026-05-25T11:40Z), Genesis takes either voice OR text role description. Phase 1 ships the textarea path; this brief adds the voice path. Wires the already-MERGED `argo-voice-io` crate (Track P P-03 — #2209) into the `/genesis` SPA surface.

The investor moment: operator says *"Hey Skycot, hire me a customer-success lead who triages tier-1 tickets and escalates refunds over $500"* — and the on-screen text fills in live as they speak, then Genesis fires from the transcript.

## Scope

1. **SPA surface**: on `/genesis`, add a microphone toggle next to the textarea. When pressed: `argo-voice-io` STT (speech-to-text) provider streams transcribed text into the textarea in real-time. Live "..." indicator while transcribing.
2. **Provider**: use the existing voice-io provider registry. Default to the deterministic stub provider in dev/demo modes; configurable to real OpenAI Realtime / Cartesia / Deepgram in production via voice-io manifest (TOML).
3. **Submit-on-stop**: pressing the mic toggle a second time stops recording AND fires Genesis with the final transcript. Operator can edit transcript before submit if they prefer.
4. **Failure mode**: mic permission denied → fall back to text input with a clear "Mic unavailable — type instead" message.
5. **Cross-platform**: tested on macOS (primary target) + Chrome dev container; iOS/Android Safari support is out-of-scope for v1.0 demo.

## Acceptance criteria

- [ ] Microphone button on `/genesis` triggers a real `argo-voice-io` STT session.
- [ ] Transcript appears in textarea live as operator speaks (partial-result rendering, not just end-of-utterance).
- [ ] Pressing mic-stop fires Genesis with the final transcript automatically (or operator can edit + manual-submit).
- [ ] Mic-denied fallback path renders a clear text-only message + still allows textarea entry.
- [ ] Function-side: a sentence like "hire me a customer success lead who triages tier-1 tickets" produces a Genesis output that includes those concepts (triage, tier-1, customer success) in the generated charter / persona.
- [ ] Cross-tenant: voice-io sessions are tenant-scoped; tenant A's transcription log is not readable by tenant B.

## May-touch

- `web/components/skycot/genesis/HeroInput.tsx` (extend with mic button + voice client wiring)
- `web/lib/api/voice-io.ts` (NEW — SPA-side voice-io client for STT streaming)
- `crates/argo-cli/src/handlers/gateway/routes_voice_io.rs` (NEW — voice-io WebSocket / SSE gateway route bridging SPA ↔ `argo-voice-io` crate)
- This brief file (status updates)

## Out of scope

- TTS (the worker speaking back) — that's Track P P-06/P-07 deliverables
- Voice-driven document upload — separate brief if needed
- Voice as primary input on mobile — desktop-only for v1.0 demo

## Effort

~250-400 LOC (gateway route + SPA mic UI + voice-io client wiring + cross-tenant test). ~3-5 hours.
