# L3 — `skill-ir-backfill` (Track L Phase 1, Tier 1)

**Owner**: claude. **Depends on**: L2 MERGED.
**Authored against**: origin/main `0ec56ca5` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L3-skill-ir-backfill`.

## Scope

One-shot backfill of existing skills to the IR format. CLI surface for operators. LOC ~600. Branch: `claude-L3-skill-ir-backfill`. PR title: `feat(skill-ir): one-shot backfill of existing skills (L3)`.

**Specifics (verbatim from L0 spec)**:

- CLI: `argo skill ir backfill [--dry-run]`.
- Reads all existing `SKILL.md` files.
- For each: parse frontmatter + body, **synthesize a minimal IR**:
  - Single `action` node containing the skill body verbatim as instruction.
  - Annotations: `confidence: 1.0` (curator-authored), `sample-count: 0`, `policy: auto`.
  - Skill metadata from frontmatter copied into graph metadata.
- Writes via L2's `SkillIrStore::put`.
- Idempotent (re-running produces same hashes).
- Reports "N skills backfilled, M skipped (already have IR)".

## Files you may touch

- NEW `crates/argo-skill/src/ir/backfill.rs` — backfill logic.
- NEW `crates/argo-skill/src/ir/backfill_tests.rs` — tests.
- NEW `crates/argo-cli/src/handlers/skill_ir_backfill.rs` — CLI handler.
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: register handler. ≤5 modified lines.

## Files you must NOT touch

- L1/L2 surfaces (types.rs, store.rs) — locked.
- `crates/argo-skill/src/lib.rs` — only L2 extends version.rs.
- Any existing `SKILL.md` content — read-only.

## Acceptance criteria

- [ ] `argo skill ir backfill` runs clean against repo's existing skills.
- [ ] Idempotent — second run produces zero writes.
- [ ] `--dry-run` reports counts without writing.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L3-skill-ir-backfill`
- PR title: `feat(skill-ir): one-shot backfill of existing skills (L3)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` for full track context.
- ~600 LOC; ~45min worker time.
