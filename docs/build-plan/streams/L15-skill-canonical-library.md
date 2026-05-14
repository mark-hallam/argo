# L15-skill-canonical-library — Track L Phase 4 (Composition + library + e2e), Tier 5

**Owner**: claude. **Depends on**: L14 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L15-skill-canonical-library`.

## Scope

6 canonical sub-skills shipped as built-in IR documents that delegate to existing Rust primitives (auth-flow, document-extraction, escalation-pattern, approval-with-timeout, csv-to-records, retry-with-backoff). Loader on startup ensures each tenant has access to the canonical set. Canvas: 'Insert canonical sub-skill' picker (one-line addition to L8). **This is packaging work, not greenfield** — see L0 §L15 for the substrate-to-wrapper map.

LOC: ~700 (mostly wrapping existing primitives). Branch: `claude-L15-skill-canonical-library`. PR title: `feat(skill-ir): canonical sub-skill library (L15)`.

## Files you may touch

- NEW directory  with 6  files + :
  - 
  - 
  - 
  - 
  - 
  - 
- NEW .
- NEW .

## Files you must NOT touch

- Existing primitives (argo-mcp-connector backoff, argo-executor-document/csv, argo-orchestrator/escalation, E1-* approvals, M1 vision extractor, per-connector OAuth) — wrap, don't refactor.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] 6 canonical sub-skills load + execute end-to-end.
- [ ] Each delegates to the existing primitive (no logic duplication).
- [ ] Canvas picker wired and inserts the right sub-skill IR.
- [ ] Each has at least one integration test composing it inside a parent skill.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L15-skill-canonical-library`
- PR title: `feat(skill-ir): canonical sub-skill library (L15)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L15-skill-canonical-library for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
