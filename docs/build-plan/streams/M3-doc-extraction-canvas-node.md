# M3-doc-extraction-canvas-node - Track M (M-Doc), Phase 1, Tier 2

**Owner**: claude. **Depends on**: M1 MERGED; L8 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M3-doc-extraction-canvas-node`.

## Scope

Adds ExtractDocument node-kind to the IR (L1 backward-compatibly extended). Canvas node: pick OCR provider + paste/upload JSON schema + bind output variable. Workers running the node call M1s extractor.

LOC: ~600. Branch: `claude-M3-doc-extraction-canvas-node`. PR title: `feat(skill-ir): ExtractDocument node-kind + canvas UI (M3)`.

## Files you may touch

- NEW crates/argo-skill/src/ir/node_kinds/extract_doc.rs - new variant ExtractDocument
- NEW web/components/skycot/canvas/nodes/ExtractDocumentNode.tsx
- NEW web/components/skycot/canvas/nodes/ExtractDocumentInspector.tsx

## Files you must NOT touch

- L1 core schema (extend via additive enum variant); M1 extractor.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] IR schema extended; backward-compatible (existing IRs parse unchanged).
- [ ] Canvas node renders + edits.
- [ ] Live trace pulses the node during execution.
- [ ] 4 unit + 1 e2e.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M3-doc-extraction-canvas-node`
- PR title: `feat(skill-ir): ExtractDocument node-kind + canvas UI (M3)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claims 4 + 5 (canvas + PC tasks).
