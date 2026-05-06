# V1.4 — Local voice TUI (Whisper)

**Owner**: claude. **Depends on**: claude-tui-realtime-dashboard
(queued) merged.

## Scope

Operator at terminal speaks "approve" / "deny" / "details"
to handle pending approvals hands-free. Pure local
processing — Whisper.cpp native bindings, no cloud round-trip.

- New `argo voice` TUI mode — listens via mic input, runs
  whisper.cpp locally, matches recognized phrase against
  pending-approval set
- Phrase grammar: `approve <worker-name>` / `deny <worker-name>`
  / `details <worker-name>` / `next` / `quit`
- Worker name fuzzy-matched against active worker list (so
  "Maria" matches "Maria Hernandez")
- Confirmation prompt for any approve/deny
  ("approve Maya for $25 charge — say YES to confirm")
- Falls back to TUI list view if no mic permission

Why local: privacy (operator's voice never leaves the box),
no API cost, works offline.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-voice-tui/` (NEW crate)
- `crates/argo-voice-tui/Cargo.toml` (NEW —
  whisper-rs + cpal deps)
- `crates/argo-voice-tui/src/{lib,recognizer,grammar,intent}.rs` (NEW)
- `crates/argo-cli/src/handlers/voice.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly:
  register `Voice` subcommand
- `Cargo.toml` workspace — narrowly: add new member

## Acceptance criteria
- [ ] Whisper.cpp binding loads + transcribes mic input
- [ ] All 5 grammar phrases recognized
- [ ] Fuzzy worker-name match (within edit distance 2)
- [ ] Confirmation prompt for approve/deny actions
- [ ] Mic-permission denial degrades to TUI list cleanly
- [ ] Wake-word ("Argo") gates listening (battery / privacy)
- [ ] 8 unit tests (grammar + fuzzy match) + 1 integration
      test gated on `ARGO_VOICE_INTEGRATION=1`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-voice-local-tui-whisper`
- PR title: `voice: local TUI voice approvals via Whisper.cpp (claude-voice-local-tui-whisper)`
