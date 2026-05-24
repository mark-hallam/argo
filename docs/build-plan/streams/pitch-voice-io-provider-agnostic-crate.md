# pitch-voice-io-provider-agnostic-crate

**Owner**: claude.
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-voice-io-provider-agnostic-crate`.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-03 of 8**

New crate `argo-voice-io` — provider-agnostic speech-to-text (STT — turning audio into words in real time), text-to-speech (TTS — turning words back into audio), and turn-taking primitives for agents that need to speak and listen in a live channel. The Pitch Agent's three channels (Rehearsal, Web Embed, Live Meeting) all consume this crate; their differences live in channel adapters, not in voice primitives.

Distinct from the existing `argo-voice-tui` crate, which is operator-side offline approval (whisper.cpp + cpal mic + intent recognition). This crate is *agent-side* online voice I/O for *production speaking*. The two solve different problems and remain separate crates.

## Why this brief exists

The v1.0 demo storyboard requires the Pitch Agent to speak naturally with sub-600ms first-response latency, handle interruptions mid-utterance, and remain provider-agnostic (operator memory: "voice as manifest parameter, not hardcoded").

Today Skycot has no agent-side voice primitive. This brief adds the foundation.

## Source of truth

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scene 2a (slide-drive + interrupt + clean resume) is the load-bearing demo of this primitive
- **Memory**: `project_pitch_agent_v1_demo.md` — "Provider-agnostic via manifest. Candidate vendors: OpenAI Realtime API, ElevenLabs+Deepgram, Cartesia Sonic. Tenant chooses; platform stays agnostic."
- **Platform invariant**: "Provider manifests, not adapters" (invariant #4) — this crate defines a trait + manifest format; vendor wiring is per-provider, not hard-coded
- **Existing related crate**: `crates/argo-voice-tui` — separate concern (operator-side offline approvals), do NOT merge

## Scope

### 1. Create the crate

```
crates/argo-voice-io/
├── Cargo.toml
├── src/
│   ├── lib.rs                  // re-exports + manifest loader
│   ├── session.rs              // VoiceSession lifecycle
│   ├── provider.rs             // VoiceProvider trait
│   ├── turn_taking.rs          // TurnTakingPolicy + interrupt detection
│   ├── manifest.rs             // TOML manifest schema + loader
│   └── providers/
│       ├── mod.rs
│       └── stub.rs             // synchronous in-memory provider for tests
```

Add to workspace `Cargo.toml`. Description: `"Argo — provider-agnostic agent-side voice I/O (STT + TTS + turn-taking) for live channels (pitch-agent, voice-meeting)."`

### 2. Public surface

```rust
/// A single voice session — open, exchange audio, close.
/// Owned by a channel adapter (Rehearsal / WebEmbed / LiveMeeting).
pub struct VoiceSession {
    session_id: SessionId,
    tenant_id: TenantId,
    provider: Box<dyn VoiceProvider>,
    turn_taking: TurnTakingPolicy,
    // ...
}

impl VoiceSession {
    pub async fn open(manifest: &VoiceManifest, tenant_id: TenantId) -> Result<Self>;
    pub async fn close(self) -> Result<()>;

    /// Stream user audio in; receive interim + final transcripts.
    pub fn transcript_stream(&mut self) -> impl Stream<Item = Result<TranscriptEvent>>;

    /// Push agent text; receive synthesized audio + send-events.
    pub async fn speak(&mut self, text: &str, opts: SpeakOptions) -> Result<SpeakHandle>;

    /// Stop the current utterance immediately (for interrupts).
    pub async fn interrupt(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum TranscriptEvent {
    Partial { text: String, is_speaker_continuing: bool },
    Final { text: String, ended_at: Instant },
    SilenceDetected { duration_ms: u32 },
    UserInterrupted,  // user started speaking while agent was speaking
}

pub struct SpeakOptions {
    pub voice_id: Option<String>,     // provider-specific
    pub interruptible: bool,          // default true
    pub priority: SpeakPriority,
}

pub enum SpeakPriority {
    Normal,
    BargeOver,  // continue even if user starts speaking; rare, e.g. emergency stop
}

pub struct SpeakHandle {
    pub utterance_id: String,
    // future-of: spoken-complete | interrupted | failed
}

/// The trait every voice provider implements.
pub trait VoiceProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> VoiceCapabilities;

    /// Open the provider-specific session (websocket, HTTP/2, etc.).
    fn open<'a>(&'a self, cfg: &'a ProviderConfig) -> BoxFuture<'a, Result<Box<dyn VoiceTransport>>>;
}

pub struct VoiceCapabilities {
    pub supports_interrupt: bool,
    pub supports_barge_in: bool,
    pub typical_first_audio_latency_ms: u32,
    pub typical_transcription_latency_ms: u32,
}

pub trait VoiceTransport: Send + Sync {
    fn push_audio<'a>(&'a mut self, pcm_frames: &'a [i16]) -> BoxFuture<'a, Result<()>>;
    fn poll_transcript(&mut self) -> Option<TranscriptEvent>;
    fn synth<'a>(&'a mut self, text: &'a str, opts: &'a SpeakOptions) -> BoxFuture<'a, Result<SpeakHandle>>;
    fn cancel_synth<'a>(&'a mut self) -> BoxFuture<'a, Result<()>>;
}
```

### 3. Manifest format

Per the "provider manifests, not adapters" invariant:

```toml
# voice-manifests/openai-realtime.toml
id = "openai-realtime"
provider = "openai-realtime"

[stt]
language = "en-AU"
interim_results = true

[tts]
voice_id = "alloy"
speed = 1.0

[turn_taking]
endpoint_silence_ms = 600
allow_user_interrupt = true
allow_agent_barge = false
```

Loader: `VoiceManifest::from_path(p: &Path) -> Result<VoiceManifest>`. Validates the provider name exists in the registry.

### 4. Provider registry + plugin-subprocess interface

Per platform invariant #1 (polyglot core; plugins as subprocesses), real providers (OpenAI Realtime, ElevenLabs+Deepgram, Cartesia) ship as subprocess plugins over a JSON-RPC line protocol — they don't link into `argo-voice-io` directly. This brief adds the trait + the registry + a `stub` in-process provider for tests. The first real provider (probably Cartesia, lowest latency) ships in a follow-up brief sized to ~600 LOC.

```rust
pub struct VoiceProviderRegistry {
    by_name: HashMap<&'static str, Arc<dyn VoiceProvider>>,
}

impl VoiceProviderRegistry {
    pub fn with_stub() -> Self;  // for tests
    pub fn register(&mut self, p: Arc<dyn VoiceProvider>);
    pub fn resolve(&self, name: &str) -> Option<Arc<dyn VoiceProvider>>;
}
```

### 5. Turn-taking policy

`TurnTakingPolicy` is a small state machine:

```rust
pub struct TurnTakingPolicy {
    pub endpoint_silence_ms: u32,
    pub allow_user_interrupt: bool,
    pub allow_agent_barge: bool,
}

#[derive(Debug, Clone)]
pub enum TurnEvent {
    UserBeganSpeaking,
    UserStoppedSpeaking,
    AgentBeganSpeaking,
    AgentFinishedSpeaking,
    InterruptionDetected { who_was_speaking: Speaker },
}

pub enum Speaker { User, Agent }
```

Logic is driven by `TranscriptEvent`s — when `UserInterrupted` is emitted while the agent is mid-`SpeakHandle`, the session auto-issues `interrupt()` and surfaces `TurnEvent::InterruptionDetected`.

### 6. Tests

`crates/argo-voice-io/src/lib_tests.rs` — using the stub provider:

- Open + close session
- Push synthetic audio frames → receive interim then final transcript events
- Speak + interrupt mid-utterance — confirm `interrupt()` cancels synth
- User-interruption auto-detection: agent speaking, user starts → `InterruptionDetected`
- Multi-tenant: two sessions, different tenant IDs, no cross-talk
- Manifest loader: valid + invalid TOML
- Provider registry: resolve known name; reject unknown

Aim for ≥10 tests. No network I/O in tests.

## Files you may touch

- `crates/argo-voice-io/Cargo.toml` (new)
- `crates/argo-voice-io/src/**` (new)
- `Cargo.toml` (workspace — add the new member)
- `voice-manifests/openai-realtime.toml` (example, new)
- `voice-manifests/cartesia.toml` (example, new — placeholder values, real cfg in follow-up)
- `voice-manifests/elevenlabs-deepgram.toml` (example, new — placeholder values)

## Files you must NOT touch

- `crates/argo-voice-tui/` — separate concern (offline operator approvals); merging would violate the "no parallel abstractions" rule by collapsing two domains
- `crates/argo-agent-runner/` — channel adapters (P-06, P-07) consume this crate; the runner doesn't directly
- Any real-provider integration code (no `reqwest` calls, no API keys read from env) — provider integrations are follow-up briefs sized for individual claim

## Acceptance criteria

- [ ] `argo-voice-io` crate builds standalone
- [ ] Stub provider lets ≥10 tests pass without network I/O
- [ ] Public surface (`VoiceSession`, `VoiceProvider`, `VoiceTransport`, `TranscriptEvent`, `SpeakOptions`, `SpeakHandle`, `TurnTakingPolicy`, `VoiceManifest`, `VoiceProviderRegistry`) exported from `lib.rs`
- [ ] Three example manifests parse cleanly
- [ ] Multi-tenant: tenant_id plumbed through every session API
- [ ] Interrupt API + user-interrupt auto-detection both work in tests
- [ ] No file >500 LOC; no fn >80 LOC; no `unwrap`/`expect` outside `#[cfg(test)]`
- [ ] Rustdoc on every public type/fn with at least one example (per core-platform invariant)

## Verification

```bash
cargo build -p argo-voice-io
cargo test -p argo-voice-io
cargo doc -p argo-voice-io --no-deps
bash scripts/check-brief-freshness.sh pitch-voice-io-provider-agnostic-crate
```

## Cross-references

- **Consumers**: `pitch-skycot-web-embed-channel` (P-06), `pitch-meeting-orchestrator-zoom-channel` (P-07), `pitch-agent-worker-template` (P-05)
- **Sibling**: `pitch-canonical-qa-skill` (P-04) — the hot-path skill produces `SpeakOptions` carrying `declared_classes` for P-02's truth-boundary gate
- **Follow-up briefs (NOT part of P-03)**: `pitch-voice-provider-cartesia`, `pitch-voice-provider-openai-realtime`, `pitch-voice-provider-elevenlabs-deepgram` — real provider subprocess plugins, ~600 LOC each
- **Memory**: `project_pitch_agent_v1_demo.md`
- **Pre-flight grep** 2026-05-24 confirmed: no existing `VoiceSession`, `VoiceProvider`, `VoiceTransport`, `TurnTakingPolicy` surfaces in `crates/`
