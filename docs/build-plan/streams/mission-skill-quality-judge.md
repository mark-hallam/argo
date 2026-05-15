# mission-skill-quality-judge — Skill quality trend tracking + auto-flag declining skills

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-quality-judge`.

## Scope

v2.0 self-improvement. Per-skill quality score trend (success/latency/cost/feedback weighted). Linear-regression slope; auto-flag if slope < -0.01. Dashboard line chart.

LOC: ~350.

## Files you may touch

- `crates/argo-skill/src/quality.rs` — narrowly: QualityTrend + slope math. ≤120 modified lines.
- Test sibling — trend computation, slope calc. ≤100 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_skill_quality.rs` (NEW) — `/api/skills/<id>/quality-trend` endpoint. ≤120 LOC.
- `crates/argo-forge/src/promote.rs` (NEW) — narrowly: demote on negative trend. ≤30 modified lines.

## Files you must NOT touch

- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Implementation matches scope above.
- [ ] All new tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-skill-quality-judge`
- PR title: `feat: Skill quality trend tracking + auto-flag declining skills (mission-skill-quality-judge)`

## Notes for the implementer

- Priority: v2.0 self-improvement transparency.
- Authored as part of post-v1 backlog batch 2026-05-15.
