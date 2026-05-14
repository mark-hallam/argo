# M13-skill-sdk-rust - Track M (M-Code), Phase 2, Tier 3

**Owner**: claude. **Depends on**: L1 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M13-skill-sdk-rust`.

## Scope

Same fluent API as M12 but in Rust, for in-process workers / vertical packs shipped as crates. Useful for canonical sub-skill library (L15) - those can be authored via SDK.

LOC: ~900. Branch: `claude-M13-skill-sdk-rust`. PR title: `feat(sdk): Rust skill SDK (M13)`.

## Files you may touch

- NEW crate crates/argo-skill-sdk/
- NEW crates/argo-skill-sdk/src/lib.rs - fluent Rust builder
- NEW crates/argo-skill-sdk/src/builders.rs
- NEW crates/argo-skill-sdk/tests/builders.rs

## Files you must NOT touch

- L1 types (consume).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Same five samples as M12 author cleanly.
- [ ] Compiles to IR matching the L1 schema.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M13-skill-sdk-rust`
- PR title: `feat(sdk): Rust skill SDK (M13)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 3.
