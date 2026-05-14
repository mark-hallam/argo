# M2-doc-airgapped-paddleocr - Track M (M-Doc), Phase 1, Tier 4 DEFERRED

**Owner**: claude. **Depends on**: M1 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M2-doc-airgapped-paddleocr`.

## Scope

DEFERRED - author only when first airgapped/sovereign customer asks. PaddleOCR for OCR (CPU-runnable; GPU much faster). Local LLM (Qwen vision or similar) for schema-conformant extraction. Provider trait from M1 makes this pluggable per-tenant via config.

LOC: ~900. Branch: `claude-M2-doc-airgapped-paddleocr`. PR title: `feat(doc): airgapped PaddleOCR fallback (M2 DEFERRED)`.

## Files you may touch

- NEW crates/argo-executor-document/src/extract/paddleocr.rs
- NEW crates/argo-executor-document/src/extract/paddle_extract.rs - two-stage pipeline
- NEW deploy/paddleocr/ - self-host deployment recipe

## Files you must NOT touch

- M1 provider trait (consume, dont change).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Same 3 fixtures pass with > 90% field accuracy.
- [ ] Deploy recipe brings up the stack on a fresh VM in < 15 min.
- [ ] No outbound network required during inference.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M2-doc-airgapped-paddleocr`
- PR title: `feat(doc): airgapped PaddleOCR fallback (M2 DEFERRED)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 5 (airgapped deployments).
