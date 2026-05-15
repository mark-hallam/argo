# mission-spend-anomaly-alerts — Rolling-window LLM spend anomaly alerts

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-spend-anomaly-alerts`.

## Scope

v1.1 production hardening. Monitor per-session spend (tokens × rate) and alert if burn rate spikes >threshold% within configurable window (default 150% over 1h baseline). Wire into existing `SpendGate<T>` after provider-call commit.

LOC: ~280.

## Files you may touch

- `crates/argo-runtime/src/spend_tracking.rs` (NEW) — rolling-window accumulator. ≤180 LOC.
- `crates/argo-provider/src/spend_gate.rs` — narrowly: hook accumulator on post-commit path (after `SpendGate<T>::record_spend`). ≤25 modified lines.
- `crates/argo-provider/src/transport.rs` — narrowly: ensure spend_gate invocation passes session_id to accumulator. ≤15 modified lines.
- `crates/argo-channels/src/notify.rs` (NEW) — narrowly: emit alert. ≤25 modified lines.
- Test sibling — window edge cases + rolling sum + spike detection. ≤100 LOC.

## Files you must NOT touch

- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Implementation matches scope above.
- [ ] All new tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-spend-anomaly-alerts`
- PR title: `feat: Rolling-window LLM spend anomaly alerts (mission-spend-anomaly-alerts)`

## Notes for the implementer

- Priority: v1.1 cost control.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
