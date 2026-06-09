# mission-genesis-voice-input-via-argo-voice-io

**Owner**: claude.
**Authored against**: origin/main `7c81986c2` on `2026-06-10` (REVIVED — originally authored against `a85963b34` on 2026-05-25).
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-voice-input-via-argo-voice-io`.
**Depends on**: SATISFIED — `mission-genesis-gateway-route-streaming-api` (#2309) + `mission-genesis-spa-page-describe-and-watch` (#2326) are both MERGED on main; `/genesis` page + `web/components/skycot/genesis/` exist.

🔴 **P1 MISSION CLAIM 3 — Phase 2 Onboarding Studio — voice-driven role description.**

Per memory [[genesis-is-ai-teammate-onboarding-studio]] (operator-clarified 2026-05-25T11:40Z), Genesis takes either voice OR text role description. The textarea path is live; this brief adds the mic path on `/genesis`.

The investor moment: operator says *"Hey Skycot, hire me a customer-success lead who triages tier-1 tickets and escalates refunds over $500"* — the on-screen text fills in as they speak, then Genesis fires from the transcript.

## REVIVED 2026-06-10 — what changed since 2026-05-25 (verified read-only)

The blocking dependencies merged, and the platform shipped MOST of the plumbing this brief originally planned to create. **The remaining gap is SPA-side only.** Verified on `7c81986c2`:

1. **Gateway voice routes EXIST — do NOT create `routes_voice_io.rs`.** The original may-touch listed it as NEW; it shipped with the live-voice V2 layer: `crates/argo-cli/src/handlers/gateway/routes_voice_io.rs` exposes `POST /api/v1/voice/{tts,stt,pitch-turn}` over the `argo-voice-io` traits, with `routes_voice_io/{openai,provider}.rs` + `routes_voice_io_tests.rs` (tenant-scoped provider). **REUSE `POST /api/v1/voice/stt`** (base64 audio in → transcript out). No new gateway surface.
2. **A voice-as-document leg also exists** (`routes_genesis_ingest_voice.rs`: spoken narration → onboarding document via `POST /api/v1/genesis/ingest-document`, transcript-or-base64 paths, MockStt offline seam). That is the SOP-upload leg — adjacent, NOT this brief; do not duplicate it. This brief is the role-DESCRIPTION mic (textarea dictation).
3. **SPA voice clients EXIST — extend, don't fork.** `web/lib/api/voice.ts` (pitchTurn / blobToBase64 / createRealtimeSession / postVoiceTranscriptTurn), `web/lib/api/use-voice-turn.ts` (tap-to-talk MediaRecorder mechanics), `web/lib/api/use-realtime-voice.ts` (WebRTC live mode for /pitch + /coach). The original may-touch's NEW `web/lib/api/voice-io.ts` would be a parallel client — **superseded: extend `voice.ts`** with a thin `sttTranscribe()` wrapper for `POST /api/v1/voice/stt` and reuse `blobToBase64` + the recorder mechanics pattern from `use-voice-turn.ts`.
4. **`HeroInput.tsx` exists as expected** (presentational textarea + CTA; parent `web/app/(app)/genesis/page.tsx` owns value/busy/submit state) — the mic toggle extends HeroInput, the transcript wiring lands in `page.tsx`.
5. **Honest AC amendment — chunked, not token-streamed, partials.** `POST /api/v1/voice/stt` is clip-request/response, not a streaming socket. "Transcript appears live as the operator speaks" is delivered via `MediaRecorder` timeslice chunks (e.g. 3-5 s) each posted to `/voice/stt` and APPENDED to the textarea — near-live dictation with the existing truth-bounded gateway route. True token-level partials would require the OpenAI Realtime WebRTC path (`use-realtime-voice.ts`), which is the LLM-conversation surface for /pitch + /coach, not dictation — OUT of scope here (note: per that hook's header, realtime mode has no server-side `evaluate_claim` post-check, another reason dictation stays on the deterministic `/voice/stt` route).

## Scope

1. **Mic toggle on `/genesis`**: extend `HeroInput.tsx` with a microphone button beside the textarea. Pressed: `getUserMedia` → `MediaRecorder` with timeslice; each chunk → `blobToBase64` → `sttTranscribe()` (`POST /api/v1/voice/stt`) → transcript appended into the controlled textarea value (via the existing `onChange` prop — page.tsx stays the state owner). Recording indicator while live.
2. **Stop-and-review**: pressing the mic toggle again stops recording; the operator can edit the accumulated transcript, then submits with the existing Generate CTA (auto-fire on stop is NOT required — keep the human-review step; min-length gate `MIN_ROLE_LEN` already exists in HeroInput).
3. **Provider**: whatever `/voice/stt` is configured with (real provider in production, `MockStt`-backed in offline dev) — the SPA adds NO provider logic.
4. **Failure modes**: mic permission denied / no device → inline "Mic unavailable — type instead" message, textarea unaffected. A failed chunk transcription is skipped with a non-blocking warning (the textarea keeps prior chunks).
5. **Desktop-first** (macOS Chrome primary); mobile Safari out of scope.

## Acceptance criteria

- [ ] Mic button on `/genesis` records and round-trips real audio chunks through `POST /api/v1/voice/stt` (network tab shows the calls; no parallel STT client added — `git diff` shows `voice.ts` extended, no new `voice-io.ts`).
- [ ] Transcribed text accumulates into the role textarea while recording (chunk-interval appends), and is editable after stop.
- [ ] Mic-denied fallback renders the text-only message; typing still works.
- [ ] Vitest: component test for HeroInput mic states (idle/recording/denied) + a `sttTranscribe()` unit test with a mocked `apiPost` (happy + failed-chunk paths). `cd web && npx vitest run <new test files>` green — paste tail.
- [ ] The SPA calls go through `web/lib/api/client.ts` (`apiPost`), so tenant headers + cookie auth are inherited — assert in the unit test that the wrapper hits `/api/v1/voice/stt` via `apiPost` (tenant scoping itself is pinned server-side by `routes_voice_io_tests.rs`; no new tenant surface is added).
- [ ] `cd web && npx tsc --noEmit` clean.

## May-touch (REVISED 2026-06-10)

- `web/components/skycot/genesis/HeroInput.tsx` (extend: mic toggle + recording states)
- `web/app/(app)/genesis/page.tsx` (wire transcript appends into the role-text state it already owns)
- `web/lib/api/voice.ts` (extend: `sttTranscribe()` wrapper for `POST /api/v1/voice/stt`; reuse `blobToBase64`)
- NEW test files beside the above (`HeroInput.test.tsx` additions / `voice.test.ts` additions)
- This brief file (status updates)

Explicitly NOT touched (changed from the 2026-05-25 version): `crates/argo-cli/src/handlers/gateway/routes_voice_io.rs` (exists — reuse), `web/lib/api/voice-io.ts` (do not create — parallel client), `routes_genesis_ingest_voice.rs` (separate shipped leg).

## Out of scope

- TTS (worker speaking back) — Track P P-06/P-07.
- Voice-narrated SOP/document upload — already shipped (`routes_genesis_ingest_voice.rs`); SPA wiring for it is a separate brief if needed.
- Token-streamed partials via OpenAI Realtime — /pitch + /coach surface, not dictation.
- Mobile.

## Effort

~150-250 LOC (SPA-only: mic UI + chunk loop + stt wrapper + tests). ~2-4 hours.
