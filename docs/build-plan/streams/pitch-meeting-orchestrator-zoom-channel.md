# pitch-meeting-orchestrator-zoom-channel

**Owner**: claude.
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-meeting-orchestrator-zoom-channel`.
**Blocked by**: `pitch-truth-boundary-runtime-enforcement` (P-02) MERGED + `pitch-voice-io-provider-agnostic-crate` (P-03) MERGED + `pitch-agent-worker-template` (P-05) MERGED.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-07 of 8**

New crate `argo-meeting-orchestrator` — joins Zoom (and Google Meet, as a follow-up) as a meeting participant, owns the local audio loop (mic + speaker), drives screen-share, and registers as `ChannelKind::LiveMeeting`. Channel 3 of the storyboard. The riskiest channel because the agent speaks live to real people; the truth-boundary gate from P-02 is what makes this safe.

## Why this brief exists

The v1.0 demo storyboard's Scene 4 requires the Pitch Agent to appear as a third tile on a Zoom call, take over when the operator hands off, drive screen-share for the architecture slide, and handle the boundary moment ("ARR question") in front of a real investor without breaking flow.

Today Skycot has macOS computer-use (claim 5, per memory) but no first-class meeting-orchestrator. This brief adds it.

Operator memory: macOS is the v1 demo path for computer-use; Linux/Windows/E2B executors are DEFERRED-v1.x. This brief is macOS-only for v1.

## Source of truth

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scene 4 (handoff, screen-share, boundary moment, post-call follow-up)
- **Memory**: `project_pitch_agent_v1_demo.md`; `project_v1_strategic_decisions_2026_05_14.md` (macOS-first computer-use)
- **Existing crate**: `crates/argo-executor-computer-use-macos` — the macOS computer-use executor this brief drives
- **Voice loop**: P-03 `argo-voice-io` — the meeting orchestrator owns one `VoiceSession` per call, bridged to the system audio device
- **Worker template**: P-05 — provides the `tenant-investor-meeting` policy ID per channel

## Scope

### 1. Create the crate

```
crates/argo-meeting-orchestrator/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── session.rs              // MeetingSession lifecycle
│   ├── provider.rs             // MeetingProvider trait
│   ├── audio_bridge.rs         // mic + speaker bridge to VoiceSession
│   ├── screen_share.rs         // initiate + drive share
│   └── providers/
│       ├── mod.rs
│       └── zoom_macos.rs       // Zoom client driving via computer-use-macos
```

Workspace member, description: `"Argo — meeting orchestrator: joins live calls (Zoom, Meet) as a participant; bridges audio to argo-voice-io and screen-share to computer-use."`

### 2. Public surface

```rust
pub struct MeetingSession {
    session_id: SessionId,
    tenant_id: TenantId,
    voice: VoiceSession,                 // from P-03
    provider: Box<dyn MeetingProvider>,
    display_name: String,                // "Pitch · SKYCOT Worker"
    avatar_url: Option<String>,
}

impl MeetingSession {
    pub async fn join(
        invite: MeetingInvite,
        cfg: MeetingConfig,
        voice: VoiceSession,
    ) -> Result<Self>;
    pub async fn leave(self) -> Result<()>;
    pub async fn start_screen_share(&mut self) -> Result<ScreenShareHandle>;
    pub async fn stop_screen_share(&mut self) -> Result<()>;
    pub fn participants(&self) -> Vec<Participant>;
}

pub struct MeetingInvite {
    pub provider_id: &'static str,       // "zoom-macos" | "meet-macos" (later)
    pub url: String,                     // zoom meeting URL or meet code
    pub display_name: String,
}

pub struct MeetingConfig {
    pub audio_device: AudioDevice,
    pub video_enabled: bool,             // for v1: false; tile shows static branded image
    pub allow_screen_share: bool,
    pub camera_image_path: Option<PathBuf>, // branded still-image for the tile
}

pub trait MeetingProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn join<'a>(&'a self, invite: &'a MeetingInvite, cfg: &'a MeetingConfig) -> BoxFuture<'a, Result<MeetingHandle>>;
    fn supports_screen_share(&self) -> bool;
}
```

### 3. Zoom-macOS provider (v1 scope)

`providers/zoom_macos.rs` uses `argo-executor-computer-use-macos` to:

1. Open Zoom desktop client via `open zoommtg://...` URL handler
2. Navigate the "Join meeting" dialog if needed
3. Set the display name to the configured value
4. Toggle audio on (using the system audio device set by P-03's voice session)
5. Detect when the meeting is connected (look for the "Leave" button via Accessibility API)
6. Optionally start screen-share via menu-driven click sequence

This is mechanical computer-use work — well-suited to the existing macOS executor. The provider trait stays clean so a Google Meet provider can follow (deferred to v1.x).

### 4. Audio bridge

The crux: piping audio between the meeting client and `argo-voice-io`'s `VoiceSession`.

- **Capture path** (other-participants → agent ear): we use a virtual audio device (BlackHole or Loopback — operator picks; flag in PR) to route the Zoom app's output into our STT input.
- **Playback path** (agent mouth → other participants): we set Zoom's microphone input to a virtual device into which we write our TTS PCM frames.
- The virtual-audio install/configuration is a one-time operator step; the runbook lives in `docs/skycot/demos/v1-pitch-agent-runbook.md` (new doc in this brief).

Cross-platform concerns are deferred. Linux/Windows audio bridging is a separate follow-up.

### 5. Screen-share driver

`screen_share.rs` exposes:

```rust
pub struct ScreenShareHandle {
    pub session_id: SessionId,
    pub source: ScreenShareSource,
}

pub enum ScreenShareSource {
    Window { app: String, title: String },
    Display { index: u32 },
    Region { x: i32, y: i32, w: u32, h: u32 },
}
```

For Scene 4a the agent shares a Keynote window — `ScreenShareSource::Window { app: "Keynote", title: "SKYCOT v1.0" }`. The driver uses computer-use to click through Zoom's share-window picker.

### 6. Channel adapter registration

Add `MeetingChannel` to `argo-channels`:

```rust
pub struct MeetingChannel {
    tenant_id: TenantId,
    meeting: MeetingSession,
}

impl ChannelAdapter for MeetingChannel {
    fn kind(&self) -> ChannelKind { ChannelKind::LiveMeeting }
    // ... trait methods
}
```

### 7. Post-call follow-up queue

Scene 4c shows the agent queueing a follow-up email (with approval gate) when the call ends. This is two existing skills composed:

1. `follow-up-writer` (existing or stubbed in P-05)
2. Approval gate (existing in `argo-delegate` / approval queue infrastructure)

The orchestrator simply emits a `MeetingEnded` event with a transcript ID; the worker's workflow definition (P-05) handles the rest. No new code in this brief beyond the event emission.

### 8. Operator runbook

`docs/skycot/demos/v1-pitch-agent-runbook.md` (new):

- Required permissions on the macOS binary: Accessibility + Screen Recording + Microphone + Camera (Microphone is the bridge; Camera only if video tile is enabled)
- Virtual-audio install instructions (BlackHole 2ch or Loopback)
- Zoom app config: display name, virtual mic/speaker, video off / camera image
- Smoke-test sequence before recording
- Failure modes (mic/speaker swapped, share-screen permission missing) + fixes

Per memory `project_v1_demo_mp4_ship_gate`, the operator owns this runbook execution. The runbook in this brief is the working draft.

### 9. Tests

`crates/argo-meeting-orchestrator/src/lib_tests.rs`:

- Session lifecycle: open + join (mock provider) + leave
- Audio bridge: PCM in → STT mock → transcript out; agent TTS → PCM out → meeting mic mock
- Screen-share: start + stop, captures the right source
- Multi-tenant: two sessions in flight, no cross-talk
- Provider trait: register + resolve

Real Zoom integration tests are manual (the runbook covers them); CI tests use mock providers.

## Files you may touch

- `crates/argo-meeting-orchestrator/**` (new)
- `Cargo.toml` (workspace — add member)
- `crates/argo-channels/src/meeting.rs` (new — channel adapter)
- `crates/argo-channels/src/lib.rs` (re-export)
- `crates/argo-channels/tests/meeting_e2e.rs` (new)
- `docs/skycot/demos/v1-pitch-agent-runbook.md` (new)

## Files you must NOT touch

- `crates/argo-executor-computer-use-macos/` — consume only; do NOT add Zoom-specific helpers there (the abstraction belongs in `argo-meeting-orchestrator`)
- `crates/argo-voice-io/` — P-03 owns it
- The audit pipeline beyond emitting standard events
- The Linux/Windows computer-use crates — they stay DEFERRED-v1.x per operator decision

## Acceptance criteria

- [ ] `argo-meeting-orchestrator` builds standalone
- [ ] Mock provider lets all unit tests pass without driving a real Zoom client
- [ ] Audio bridge round-trip (PCM in → transcript; text in → PCM out) works in tests
- [ ] Screen-share API exposes window/display/region sources
- [ ] Channel adapter registers as `ChannelKind::LiveMeeting`
- [ ] Multi-tenant negative test passes
- [ ] Runbook is reviewable by the operator (no hand-wavy steps; commands explicit)
- [ ] No file >500 LOC; no fn >80 LOC

## Verification

```bash
cargo build -p argo-meeting-orchestrator -p argo-channels
cargo test -p argo-meeting-orchestrator
cargo test -p argo-channels meeting
bash scripts/check-brief-freshness.sh pitch-meeting-orchestrator-zoom-channel
```

Manual (operator-only — requires macOS perms + Zoom + virtual audio):
1. Follow `docs/skycot/demos/v1-pitch-agent-runbook.md` setup steps
2. Start a Zoom meeting from a second device
3. Run `argo pitch join <zoom-url>`
4. Confirm Pitch tile appears, joins audio, can be spoken to and respond
5. Trigger screen-share via the SPA workflow controller
6. Trigger the ARR question → confirm refusal + audit row

## Cross-references

- **Blocked by**: P-02, P-03, P-05
- **Parallel**: `pitch-skycot-web-embed-channel` (P-06) — sibling channel
- **Memory**: `project_pitch_agent_v1_demo.md`, `project_v1_strategic_decisions_2026_05_14.md`, `project_v1_demo_mp4_ship_gate.md`
- **Storyboard**: Scene 4a (handoff + screen-share), 4b (boundary in front of investor), 4c (post-call follow-up queue)
- **Pre-flight grep** 2026-05-24: no existing `MeetingSession`/`MeetingProvider`/`MeetingChannel` surfaces in `crates/`
- **Deferred follow-ups (NOT in this brief)**: Google Meet provider, Linux audio bridge, Windows audio bridge, video tile with a generated talking-head avatar
