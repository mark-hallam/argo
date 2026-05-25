# fix-pitch-agent-worker-template-may-touch-refresh — widen may-touch on Track P P-05 to unblock claim

**Owner**: claude or codex. **Depends on**: nothing (refresh-only).
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pitch-agent-worker-template-may-touch-refresh`.

## Why this is P1

Track P P-05 brief `pitch-agent-worker-template` is BLOCKED because the may-touch list is too narrow: the brief requires new modules `crates/argo-skill/src/pitch_mode_switcher.rs` + `slide_controller.rs`, but `crates/argo-skill/src/lib.rs` has explicit `mod` declarations and is NOT in may-touch. The new modules cannot compile without violating the absolute file-boundary rule.

CLAUDE.md notes that PR #2293 (P-05 brief refresh) MERGED 2026-05-25T10:40Z widened may-touch to cohorts registry but the ledger flip BLOCKED→TODO "never landed". The brief on disk needs another refresh to either (a) add `crates/argo-skill/src/lib.rs` to may-touch, OR (b) move the skills to pack-local `.skill.json` / TOML so no Rust module wiring is needed. Operator-locked v1.0 demo gate is downstream.

## Scope

1. Read `docs/build-plan/streams/pitch-agent-worker-template.md` to confirm the current may-touch list.
2. Read PR #2293's diff to understand what was widened previously.
3. Decide path (a) vs (b):
   - (a) **Add `crates/argo-skill/src/lib.rs` to may-touch**: smallest change, most direct, but slightly widens the boundary. Recommended if `argo-skill` is the natural home.
   - (b) **Move skills to pack-local artefacts**: more invasive but follows the operator-stated "pack-local skills" pattern. May not be possible if the skill needs Rust-side runtime logic (mode switcher, slide controller).
4. Recommend (a) for minimal change unless mode-switcher behaviour is purely declarative.
5. Edit `docs/build-plan/streams/pitch-agent-worker-template.md` in-place:
   - Refresh freshness header to `289d3077f` / `2026-05-25`.
   - Add `crates/argo-skill/src/lib.rs` to may-touch (path a).
   - Add explicit module-wiring step to the scope ("add `pub mod pitch_mode_switcher;` + `pub mod slide_controller;` to `argo-skill/src/lib.rs`").
6. Flip the BLOCKED ledger row to TODO with a note pointing at this refresh PR's merge SHA.

## Files you may touch

- `docs/build-plan/streams/pitch-agent-worker-template.md` — refresh in-place.
- `docs/build-plan/BUILD_STATUS.md` — flip BLOCKED → TODO on the row (on this brief's merge).

## Files you must NOT touch

- Anything under `crates/`.

## Acceptance criteria

- [ ] `pitch-agent-worker-template` may-touch list includes `crates/argo-skill/src/lib.rs`.
- [ ] Brief freshness header points at `289d3077f` or later.
- [ ] BUILD_STATUS.md row for `pitch-agent-worker-template` flips from BLOCKED to TODO (post-merge, in the same commit).
- [ ] Diff is docs-only.

## Branch + PR

- Branch: `claude-fix-pitch-agent-worker-template-may-touch-refresh` or `codex-…`.
- PR title: `track-p(p-05): widen may-touch on agent worker template (fix-pitch-agent-worker-template-may-touch-refresh)`.

## Notes

- Docs-only. Fast-mode candidate.
- Under the overnight Track P merge-aggressive window — `--skip-clippy` authorized.
