# L2 — `skill-ir-storage` (Track L Phase 1, Tier 1)

**Owner**: claude. **Depends on**: L1 MERGED.
**Authored against**: origin/main `0ec56ca5` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L2-skill-ir-storage`.

## Scope

SQLite storage + versioned writes for the Skill IR. LOC ~700. Branch: `claude-L2-skill-ir-storage`. PR title: `feat(skill-ir): sqlite storage + versioned writes (L2)`.

**Specifics (verbatim from L0 spec)**:

- Table `skill_ir(skill_id, version, ir_json, ir_hash, created_at, tenant_id)` plus `prev_hash` for chain.
- `SkillIrStore` trait + sqlite impl with these methods:
  - `put(skill_id, version, graph) -> ir_hash`
  - `get(skill_id, version) -> Option<SkillGraph>`
  - `latest(skill_id) -> Option<(version, SkillGraph)>`
  - `list_versions(skill_id) -> Vec<version>`
- All writes hash-chained against the prior version (`prev_hash` column).
- Multi-tenant: `tenant_id` required on every row.
- Extend `SkillVersion` (in `version.rs`) with optional `ir_hash` field.

## Files you may touch

- NEW `crates/argo-skill/src/ir/store.rs` — trait + sqlite impl.
- NEW `crates/argo-skill/src/ir/store_tests.rs` — 6 unit tests minimum.
- NEW `crates/argo-skill/migrations/NNNN_skill_ir.sql` — additive migration (pick next free slot at claim time; verify against `ls crates/argo-store/migrations/`).
- `crates/argo-skill/src/version.rs` — narrowly: extend `SkillVersion` with optional `ir_hash` field. ≤10 modified lines.

## Files you must NOT touch

- `crates/argo-skill/src/ir/types.rs` (locked; L1 owns).
- Any gateway or web component (later phases).

## Acceptance criteria

- [ ] Migration applies cleanly to a fresh DB.
- [ ] Migration applies cleanly on top of an existing populated DB.
- [ ] Round-trip: write graph, read back, hash matches.
- [ ] Hash-chain: tampering with row N detected by hash-mismatch in row N+1.
- [ ] 6 unit tests pass.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L2-skill-ir-storage`
- PR title: `feat(skill-ir): sqlite storage + versioned writes (L2)`

## Notes for the implementer

- Multi-tenant invariant per CLAUDE.md: `tenant_id` on every row.
- Migration slot: pick next-free at claim time; v60.rs is reserved per slot-64 cluster history.
- See `streams/L0-skill-canvas-track-overview.md` for full track context.
