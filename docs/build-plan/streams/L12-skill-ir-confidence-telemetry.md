# L12-skill-ir-confidence-telemetry — Track L Phase 3 (Observation extraction), Tier 4

**Owner**: claude. **Depends on**: L5 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L12-skill-ir-confidence-telemetry`.

## Scope

Confidence telemetry: per-node sample counts + success/failure tracking. Trace events from L5 update telemetry rows. Confidence = success_count / sample_count (with smoothing for low samples). Queryable via . Canvas annotations read live confidence from this table.

LOC: ~600. Branch: `claude-L12-skill-ir-confidence`. PR title: `feat(skill-ir): confidence + sample-count telemetry (L12)`.

## Files you may touch

- NEW .
- NEW  — additive migration (pick next-free slot at claim time).
- NEW .

## Files you must NOT touch

- L1 schema; L5 trace event format (consume, don't change).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Telemetry rows update within 5s of trace events.
- [ ] Confidence calc handles zero-sample case (returns null, not divide-by-zero).
- [ ] Tenant-isolated.
- [ ] 5 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L12-skill-ir-confidence`
- PR title: `feat(skill-ir): confidence + sample-count telemetry (L12)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L12-skill-ir-confidence-telemetry for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
