# L11-cu-recording-to-ir — Track L Phase 3 (Observation extraction), Tier 4

**Owner**: claude. **Depends on**: L1 MERGED; L2 MERGED; existing argo-executor-computer-use recording infrastructure.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L11-cu-recording-to-ir`.

## Scope

Input: CU session recording (sequence of screenshots + actions + DOM dumps). Pipeline: segment into logical steps → LLM identifies node-kind per step → build edges from order + branch detection → annotate confidence. Output: IR graph stored via L2.

LOC: ~1600. Branch: `claude-L11-cu-recording-to-ir`. PR title: `feat(skill-ir): synthesize graph from CU recording (L11)`.

## Files you may touch

- NEW .
- NEW  — LLM-driven step abstraction.
- NEW .
- NEW  — CLI handler.
- NEW  — gateway endpoint.

## Files you must NOT touch

- argo-executor-computer-use recording infra (read-only); L1 schema.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Three recorded fixtures (one per role) synthesize valid IR with ≥ 4 nodes each.
- [ ] Confidence annotations populated; low-confidence nodes flagged.
- [ ] Idempotent within same recording.
- [ ] Tenant-scoped + audit-emitting.
- [ ] 6 unit tests pass.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L11-cu-recording-to-ir`
- PR title: `feat(skill-ir): synthesize graph from CU recording (L11)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L11-cu-recording-to-ir for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).


## Files you may touch — AUTHORITATIVE 2026-05-15 evening (supersedes placeholder list)

L11 distills a CU action-recording trajectory into a `SkillGraph` IR. Current `argo-executor-computer-use/src/learn.rs` already builds a mechanical `SkillCandidate` from trajectories. This brief replaces the mechanical path with a proper IR-graph emit.

- `crates/argo-executor-computer-use/src/learn.rs` — narrowly: extend `from_cu_trajectory` (or sibling fn) to emit a `SkillGraph` instead of (or alongside) the mechanical SkillCandidate. ≤120 modified lines.
- `crates/argo-executor-computer-use/src/learn_tests.rs` — narrowly: add 3+ tests covering trajectory→graph emit. ≤100 modified lines.
- NEW `crates/argo-skill/src/ir/from_cu_trajectory.rs` — graph-emit helper module (separates the IR construction from executor concerns). ≤300 LOC.
- `crates/argo-skill/src/ir/mod.rs` — narrowly: `pub mod from_cu_trajectory;`. ≤3 modified lines.
- `crates/argo-skill/src/ir/types.rs` (488 LOC — must NOT push over 500) — narrowly: only if a new NodeKind variant is required to represent CU-specific actions; document the variant in PR description. ≤10 modified lines.

If types.rs would exceed 500 LOC, BLOCK with note requesting a `claude-skill-ir-types-split` precursor brief.
