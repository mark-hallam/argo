# L16-skill-marketplace-templates — Track L Phase 4 (Composition + library + e2e), Tier 5

**Owner**: claude. **Depends on**: L15 MERGED; existing marketplace (argo-skill/marketplace.rs).
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L16-skill-marketplace-templates`.

## Scope

Marketplace tier: skill templates published with full IR (not just ). Template = ready-to-instantiate IR + parameter slots. Operator browses, clicks 'Use template,' fills slots, canvas opens with the filled-in graph.

LOC: ~800. Branch: `claude-L16-skill-marketplace-templates`. PR title: `feat(skill-marketplace): canvas-template tier (L16)`.

## Files you may touch

- NEW .
- NEW .
- NEW  — template browser.
- NEW .

## Files you must NOT touch

- Existing marketplace.rs (extend, don't refactor); L1 schema.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Template browser shows ≥ 5 sample templates.
- [ ] Instantiation creates a working skill per tenant.
- [ ] 5 unit + 1 e2e.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L16-skill-marketplace-templates`
- PR title: `feat(skill-marketplace): canvas-template tier (L16)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L16-skill-marketplace-templates for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
