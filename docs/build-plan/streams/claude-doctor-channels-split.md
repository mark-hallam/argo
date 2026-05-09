# claude-doctor-channels-split — split doctor_channels.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep.**

**Authored against**: origin/main c2057ac1 on 2026-05-09 (conductor-authored prereq for claude-approval-channel-fallback-chain).

## Why

`crates/argo-cli/src/handlers/doctor_channels.rs` is 483 LOC — 17 LOC below the 500-LOC ceiling. `claude-approval-channel-fallback-chain` needs to add fallback-chain logic to doctor checks and would push past the ceiling.

This brief pre-emptively splits the file into per-channel-type sub-modules.

## Approach (concrete)

`doctor_channels.rs` likely contains one diagnostic function per channel type (push / slack / email / sms / webinbox / discord / whatsapp / etc.). Split it into:

- `doctor_channels.rs` — keep the top-level orchestrator + dispatch; ≤ 250 LOC.
- `doctor_channels_kinds.rs` (NEW) — per-channel diagnostic functions.

## Files you may touch

- `crates/argo-cli/src/handlers/doctor_channels.rs` — narrowly: extract per-kind diagnostic fns, re-export
- `crates/argo-cli/src/handlers/doctor_channels_kinds.rs` (NEW) — extracted content
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line

## Files you must NOT touch

- Any other doctor handler (`doctor.rs`, `doctor_h_stack.rs`, etc.)
- The test files that exercise doctor_channels behaviour — split is pure refactor

## Acceptance criteria

- [ ] `doctor_channels.rs` ≤ 250 LOC after the split
- [ ] `doctor_channels_kinds.rs` ≤ 400 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli doctor_channels` clean (no behaviour change)
- [ ] No public API changes; call sites still see the original surface via `pub use`

## Branch + PR

- Branch: `claude-doctor-channels-split`
- PR title: `cli: split doctor_channels.rs below 500-LOC ceiling (claude-doctor-channels-split)`

## Unblocks

- `claude-approval-channel-fallback-chain` — currently BLOCKED on the ceiling. After this lands, that brief also needs may-touch widening to include `argo-channels::NotifyChannelRef`.
