# M18-vertical-pack-vendor-manager - Track M (M-Tmpl), Phase 3, Tier 3

**Owner**: claude. **Depends on**: M15 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M18-vertical-pack-vendor-manager`.

## Scope

The Vendor Manager vertical pack. 5 skills: draft-rfp-response, score-vendor-proposal, negotiate-renewal, track-vendor-sla, route-procurement-approval. Procurement-focused, default_display_name=Vendor Manager.

LOC: ~1800. Branch: `claude-M18-vertical-pack-vendor-manager`. PR title: `feat(packs): Vendor Manager vertical pack (M18)`.

## Files you may touch

- NEW packs/vendor-manager/ directory (same shape)
- - pack.toml, worker.toml
- - 5 skill IR files
- - memory/seed.json (vendor policies, RFP rubric)
- - evals/*.json (>=20 fixtures)
- - README.md

## Files you must NOT touch

- M15 loader internals; L1 schema.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Pack installs cleanly.
- [ ] End-to-end demo: receive an RFP -> score proposals -> draft response.
- [ ] Eval suite green.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M18-vertical-pack-vendor-manager`
- PR title: `feat(packs): Vendor Manager vertical pack (M18)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 1 + 3.


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
