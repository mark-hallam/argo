# M1-doc-claude-vision-extraction - Track M (M-Doc), Phase 1, Tier 1

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M1-doc-claude-vision-extraction`.

## Scope

Operator-defined extraction schema (JSON Schema) -> Claude vision OCR + structured extraction in one call. Schema validation rejects non-conformant output; one retry with self-correction. Per-field confidence inferred. Audit-emitting (page count + provider + model + cost). Provider trait so PaddleOCR fallback (M2) can plug in later. Primary provider locked to Anthropic Claude vision per M0 operator-decisions section 2.

LOC: ~1100. Branch: `claude-M1-doc-claude-vision-extraction`. PR title: `feat(doc): Claude vision-based extraction (M1)`.

## Files you may touch

- NEW crates/argo-executor-document/src/extract/mod.rs
- NEW crates/argo-executor-document/src/extract/schema.rs - JSON-schema extraction targets
- NEW crates/argo-executor-document/src/extract/claude_vision.rs - Claude vision provider
- NEW crates/argo-executor-document/src/extract/provider.rs - ExtractionProvider trait
- NEW crates/argo-executor-document/src/extract/tests.rs

## Files you must NOT touch

- Existing PDF/CSV extractors (extend, dont refactor); provider manifest system.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Invoice fixture: extracts invoice number, total, line items > 95% field accuracy.
- [ ] Receipt fixture: extracts merchant, total, tax, items.
- [ ] Scanned-contract fixture: extracts party names + key dates.
- [ ] Schema-validation failures retry exactly once.
- [ ] Confidence scores populated per field.
- [ ] 8 unit + 3 fixture-based integration tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M1-doc-claude-vision-extraction`
- PR title: `feat(doc): Claude vision-based extraction (M1)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 5 (PC tasks: doc understanding).


## Amendment 2026-05-14 (may-touch widening)

Worker self-BLOCKED on initial claim noting that may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
