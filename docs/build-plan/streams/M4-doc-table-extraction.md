# M4-doc-table-extraction - Track M (M-Doc), Phase 1, Tier 2

**Owner**: claude. **Depends on**: M1 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M4-doc-table-extraction`.

## Scope

Specialized table extractor: detects table boundaries, infers headers, emits row-major JSON. Handles multi-page tables (continuation detection). Works on scanned + native PDF.

LOC: ~800. Branch: `claude-M4-doc-table-extraction`. PR title: `feat(doc): specialized table extractor (M4)`.

## Files you may touch

- NEW crates/argo-executor-document/src/extract/tables.rs
- NEW crates/argo-executor-document/src/extract/tables_tests.rs

## Files you must NOT touch

- M1 base extractor (compose, dont reimplement).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] 3 fixtures: bank statement, AR aging report, multi-page invoice line-items.
- [ ] Each extracts with > 95% row-level accuracy.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M4-doc-table-extraction`
- PR title: `feat(doc): specialized table extractor (M4)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 5 (PC tasks).


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
