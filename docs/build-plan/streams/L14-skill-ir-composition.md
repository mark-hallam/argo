# L14-skill-ir-composition — Track L Phase 4 (Composition + library + e2e), Tier 5

**Owner**: claude. **Depends on**: L7 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L14-skill-ir-composition`.

## Scope

SubSkillCall node references another skill by ID + version. Canvas renders as clickable composite node — double-click to drill into sub-skill's canvas. Runtime resolves sub-skill at execute time; cycle detection. Versioning: pinning a sub-skill version vs floating.

LOC: ~1000. Branch: `claude-L14-skill-ir-composition`. PR title: `feat(skill-ir): sub-skill-call execution + canvas (L14)`.

## Files you may touch

- NEW .
- NEW .
- NEW  — runtime resolution at execute time.
-  — TOUCH (from L8).

## Files you must NOT touch

- L1 schema; L4 viewer core.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] 3-skill composition runs end-to-end.
- [ ] Cycle detection rejects A→B→A.
- [ ] Canvas drill-down works.
- [ ] 5 unit tests + 1 e2e.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L14-skill-ir-composition`
- PR title: `feat(skill-ir): sub-skill-call execution + canvas (L14)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L14-skill-ir-composition for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).


## Files you may touch — AUTHORITATIVE 2026-05-15 evening (supersedes placeholder list)

L14 composes multiple skill IRs into a parent skill (sub-skill calls). Current NodeKind has `SubSkillCall` already (per L8 #1668 ship); composition is the logic that calls into a sub-skill's IR.

- NEW `crates/argo-skill/src/ir/composition.rs` — composition logic. ≤300 LOC.
- NEW `crates/argo-skill/src/ir/composition_tests.rs` — 5+ tests. ≤200 LOC.
- `crates/argo-skill/src/ir/mod.rs` — narrowly: `pub mod composition;`. ≤3 modified lines.
- `crates/argo-skill/src/ir/types.rs` (488 LOC) — narrowly: ONLY if SubSkillCall variant needs extension. ≤10 modified lines. (Verify SubSkillCall is sufficient before touching types.rs.)
- `web/components/skycot/canvas/nodes/SubSkillCallNode.tsx` (already exists per L8) — narrowly: render composition info if needed. ≤30 modified lines.

If types.rs would exceed 500 LOC: BLOCK with `claude-skill-ir-types-split` precursor.
