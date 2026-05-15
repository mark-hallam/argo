# mission-anomaly-detector — Worker behavior anomaly scoring + auto-pause gate

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-anomaly-detector`.

## Scope

v1.1 production hardening. Detect worker going off-rails (unusual action sequences, sudden budget burn, repeated same-task-type failures, latency spikes). Score 0–1.0; threshold-based pause gate when operator opts in via `[anomaly] auto_pause = true`.

LOC: ~750.

## Files you may touch

- NEW `crates/argo-runtime/src/anomaly.rs` (NEW) — scoring engine. ≤500 LOC.
- NEW `crates/argo-runtime/src/anomaly_tests.rs` (NEW) — synthetic anomalous trajectories. ≤200 LOC.
- `crates/argo-cli/src/handlers/work_loop.rs` — narrowly: anomaly-check gate. ≤50 modified lines.
- `crates/argo-runtime/src/lib.rs` — `pub mod anomaly;`. ≤3 lines.
- `crates/argo-config/src/sections.rs` — `[anomaly]` block. ≤25 modified lines.

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

- Branch: `codex-mission-anomaly-detector`
- PR title: `feat: Worker behavior anomaly scoring + auto-pause gate (mission-anomaly-detector)`

## Notes for the implementer

- Priority: v1.1 reliability — claim 1 trust.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
