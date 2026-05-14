# M19-vertical-pack-support-tier1 - Track M (M-Tmpl), Phase 3, Tier 3

**Owner**: claude. **Depends on**: M15 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M19-vertical-pack-support-tier1`.

## Scope

The Support Specialist (Tier 1) vertical pack. 5 skills: triage-ticket, resolve-from-kb, escalate-to-tier2, draft-resolution-message, update-ticket-status. Default_display_name=Support Specialist.

LOC: ~1800. Branch: `claude-M19-vertical-pack-support-tier1`. PR title: `feat(packs): Support Specialist (Tier1) vertical pack (M19)`.

## Files you may touch

- NEW packs/support-tier1/ directory
- - pack.toml, worker.toml
- - 5 skill IR files
- - memory/seed.json (KB index, escalation rules)
- - evals/*.json (>=20 fixtures)
- - README.md

## Files you must NOT touch

- M15 loader internals.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Pack installs cleanly.
- [ ] End-to-end demo: receive a ticket -> triage -> KB resolution OR escalate.
- [ ] Eval suite green.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M19-vertical-pack-support-tier1`
- PR title: `feat(packs): Support Specialist (Tier1) vertical pack (M19)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 1 + 3.


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
