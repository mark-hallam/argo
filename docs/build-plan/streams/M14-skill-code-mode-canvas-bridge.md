# M14-skill-code-mode-canvas-bridge - Track M (M-Code), Phase 2, Tier 3

**Owner**: claude. **Depends on**: M12 MERGED; L7 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M14-skill-code-mode-canvas-bridge`.

## Scope

Canvas -> Code: emit M12 SDK source from current IR. Code -> Canvas: parse SDK source, build IR, render. Round-trip with no information loss for the SDK-expressible subset.

LOC: ~800. Branch: `claude-M14-skill-code-mode-canvas-bridge`. PR title: `feat(web/canvas): code-mode round-trip (M14)`.

## Files you may touch

- NEW web/app/(app)/workers/[id]/skills/[skillId]/code/page.tsx
- NEW web/components/skycot/canvas/CodeView.tsx - Monaco editor
- NEW web/lib/api/skill-code.ts

## Files you must NOT touch

- M12 SDK internals; L7 canvas core.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Five sample skills round-trip canvas->code->canvas with identical IR hash.
- [ ] Syntax errors surface inline.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M14-skill-code-mode-canvas-bridge`
- PR title: `feat(web/canvas): code-mode round-trip (M14)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 3 (pro-code + canvas authoring duality).


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
