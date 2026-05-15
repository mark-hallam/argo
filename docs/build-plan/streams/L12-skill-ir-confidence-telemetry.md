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


## Files you may touch — AUTHORITATIVE 2026-05-15 evening (supersedes placeholder list)

L12 adds confidence scoring to IR nodes for downstream eval/retry decisions. Current `NodeKind` enum in `argo-skill/src/ir/types.rs` has no confidence field.

- `crates/argo-skill/src/ir/types.rs` (488 LOC near ceiling) — narrowly: add optional `confidence: Option<f64>` field to `SkillGraph` or per-node where load-bearing. ≤30 modified lines.
- `crates/argo-skill/src/ir/types_tests.rs` (488 LOC — only if room; verify at claim time) — narrowly: 3+ tests for confidence round-trip. ≤80 modified lines if room.
- `crates/argo-skill/src/ir/serde.rs` — narrowly: ensure confidence serializes with `#[serde(skip_serializing_if = "Option::is_none")]` for backward compat. ≤20 modified lines.
- NEW `crates/argo-skill/src/ir/confidence_telemetry.rs` — telemetry helper module for reading + emitting confidence signals. ≤200 LOC.
- `crates/argo-skill/src/ir/mod.rs` — narrowly: `pub mod confidence_telemetry;`. ≤3 modified lines.
- If `types.rs` or `types_tests.rs` would exceed 500 LOC: BLOCK with precursor `claude-skill-ir-types-split` request.
