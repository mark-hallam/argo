# mission-operator-feedback-channel — Thumbs up/down feedback per action + Forge signal integration

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-operator-feedback-channel`.

## Scope

v2.0 self-improvement. Operator feedback via web UI thumbs OR `argo action vote <id> <1-5>` CLI. Stored as `AuditKind::OperatorFeedback`. Forge reads feedback as eval signal — high-rated actions get higher proposal score; low-rated get lower.

LOC: ~300.

## Files you may touch

- `crates/argo-audit/src/event.rs` — narrowly: add `OperatorFeedback { action_id, rating }` variant. ≤30 modified lines.
- `crates/argo-cli/src/handlers/gateway/routes_action_feedback.rs` (NEW) — web endpoint. ≤120 LOC.
- `crates/argo-cli/src/handlers/action.rs` (NEW) — narrowly: CLI vote handler. ≤50 modified lines.
- `crates/argo-forge/src/propose.rs` (NEW) — narrowly: feedback-weighted scoring. ≤40 modified lines.
- Test sibling — feedback storage + Forge weighting. ≤100 LOC.

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

- Branch: `codex-mission-operator-feedback-channel`
- PR title: `feat: Thumbs up/down feedback per action + Forge signal integration (mission-operator-feedback-channel)`

## Notes for the implementer

- Priority: v2.0 — self-improvement signal.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
