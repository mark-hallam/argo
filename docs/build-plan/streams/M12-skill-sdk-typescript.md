# M12-skill-sdk-typescript - Track M (M-Code), Phase 2, Tier 2

**Owner**: claude. **Depends on**: L1 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M12-skill-sdk-typescript`.

## Scope

Fluent TS builder API that compiles to valid Skill IR. Publishable as @skycot/skill-sdk on npm. Five sample skills (one per role pack) author cleanly in TS.

LOC: ~1200. Branch: `claude-M12-skill-sdk-typescript`. PR title: `feat(sdk): TypeScript skill SDK (M12)`.

## Files you may touch

- NEW web/packages/skycot-skill-sdk/ - standalone npm package (package name @skycot/skill-sdk)
- NEW web/packages/skycot-skill-sdk/src/index.ts
- NEW web/packages/skycot-skill-sdk/src/builders.ts
- NEW web/packages/skycot-skill-sdk/src/types.ts - re-exports L1 TS types
- NEW web/packages/skycot-skill-sdk/test/builders.test.ts

## Files you must NOT touch

- L1 TS types (re-export, dont redefine).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Five sample skills (one per role pack) author cleanly in TS.
- [ ] All compile to valid IR matching the L1 schema.
- [ ] npm package builds + has type definitions.
- [ ] 10 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M12-skill-sdk-typescript`
- PR title: `feat(sdk): TypeScript skill SDK (M12)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 3 (Platform builds workers - pro-code authoring tier).
