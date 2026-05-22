# claude-channels-push-voice-split — split push.rs + voice.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep.**

**Authored against**: origin/main c2057ac1 on 2026-05-09 (conductor-authored prereq for claude-approval-rate-limit).

## Why

`crates/argo-channels/src/push.rs` is 499 LOC and `voice.rs` is 489 LOC. Both are 1-11 LOC below the 500-LOC project ceiling. Any addition (e.g. `claude-approval-rate-limit` adding rate-limit context) crosses the ceiling.

This brief pre-emptively splits both into sibling modules, mirroring the proven pattern from #851 (worker_actions split) and #882 (cli_subcommands split). Pure refactor, zero behaviour change.

## Approach (concrete)

For each file, identify a natural cut line (typically: types + parsing on one side, send/transport on the other). Create a sibling file at `crates/argo-channels/src/<name>_<topic>.rs` and move the chosen functions / types there. Re-export from the original file via `pub use` so call sites are unchanged.

Suggested cuts (worker may adjust):

- `push.rs` → keep `PushChannel` + send loop in `push.rs`; move helpers (config parsing, payload formatting) to `push_helpers.rs` or `push_format.rs`.
- `voice.rs` → keep `VoiceChannel` + send loop in `voice.rs`; move TTS-format / SSML helpers to `voice_format.rs`.

## Files you may touch

- `crates/argo-channels/src/push.rs` — narrowly: extract a sibling module, re-export
- `crates/argo-channels/src/push_<topic>.rs` (NEW) — extracted content
- `crates/argo-channels/src/voice.rs` — narrowly: same
- `crates/argo-channels/src/voice_<topic>.rs` (NEW) — extracted content
- `crates/argo-channels/src/lib.rs` — narrowly: pub mod declarations for the new siblings

## Files you must NOT touch

- Any other channel implementation (slack/discord/email/whatsapp/sms/webinbox/sse) — out of scope
- `argo-cli/src/handlers/channels*.rs` — call sites must remain unchanged via re-export
- Any test that calls these channels — behaviour must not change

## Acceptance criteria

- [ ] Both `push.rs` and `voice.rs` are now ≤ 480 LOC each (≥ 20 LOC headroom)
- [ ] Sibling files each ≤ 400 LOC
- [ ] `cargo clippy -p argo-channels --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-channels` clean (zero behaviour change)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (verify call sites unaffected)
- [ ] No public API changes — `pub use` re-exports preserve the surface

## Branch + PR

- Branch: `claude-channels-push-voice-split`
- PR title: `channels: split push.rs + voice.rs below 500-LOC ceiling (claude-channels-push-voice-split)`

## Notes for the implementer

- The split target depends on how the file is structured. Read each file end-to-end first.
- This is a refactor PR — every test that passed before must pass after. `pub use` makes the public API binary-compatible.

## Unblocks

- `claude-approval-rate-limit` — currently BLOCKED on push.rs/voice.rs ceiling.
