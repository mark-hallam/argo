# mission-skill-versioning-ab-regression — Auto-promote new skill versions on A/B regression detection

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-versioning-ab-regression`.

## Scope

v2.0 self-improvement. Skill versions have A/B cohort + performance metrics. Define regression as Δ_success < -2% or Δ_cost > +20%. At 100+ samples, auto-promote if no regression; demote if regression detected.

LOC: ~500.

## Files you may touch

- `crates/argo-skill/src/ab_test.rs` — narrowly: regression detector. ≤120 modified lines.
- `crates/argo-forge/src/promote.rs` — narrowly: auto-promote/demote decision. ≤80 modified lines.
- Test sibling — cohort assignment + regression math. ≤200 LOC.

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

- Branch: `codex-mission-skill-versioning-ab-regression`
- PR title: `feat: Auto-promote new skill versions on A/B regression detection (mission-skill-versioning-ab-regression)`

## Notes for the implementer

- Priority: v2.0 — claim 4 self-improvement.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
