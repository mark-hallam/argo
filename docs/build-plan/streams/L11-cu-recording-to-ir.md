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
