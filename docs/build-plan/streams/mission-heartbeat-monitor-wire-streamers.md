# mission-heartbeat-monitor-wire-streamers — wire HeartbeatMonitor::note_activity at 4 streamer call sites

**Owner**: claude. **Depends on**: `argo_provider::HeartbeatMonitor` (shipped at heartbeat.rs:26); manifest validates `heartbeat_event` + `heartbeat_grace_ms` fields.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> **Phase 3 wire-or-delete decision (CEO-authorized 2026-05-14): WIRE.** Audit cluster 1 D-3 found `HeartbeatMonitor` is type-level only. 4 streaming transport impls rely on `reqwest::timeout(read_timeout_ms)`; none call `note_activity`. Manifest validates the field for a feature that doesn't exist.

## Scope

In each of the 4 streamers (`anthropic/stream.rs`, `openai_responses/stream.rs`, `bedrock/stream.rs`, `openai_compat/stream.rs`), construct a `HeartbeatMonitor::new(grace_ms)` at stream-start + call `note_activity()` on every event chunk + check `should_abort()` between chunks. Abort with `ArgoError::Provider("heartbeat grace exceeded")` on miss.

## Files you may touch
- 4 streamer files in `crates/argo-provider/src/{anthropic,openai_responses,bedrock,openai_compat}/stream.rs` — narrowly: ≤30 LOC each.
- `crates/argo-provider/src/heartbeat.rs` — narrowly: minor API tweaks if needed.

## Acceptance
- [ ] All 4 streamers consult HeartbeatMonitor.
- [ ] Stream with 10s grace + 30s silent producer aborts at 10s.
- [ ] Workspace clippy clean. Unit + integration tests pass.

## Branch
- `claude-mission-heartbeat-monitor-wire-streamers`
