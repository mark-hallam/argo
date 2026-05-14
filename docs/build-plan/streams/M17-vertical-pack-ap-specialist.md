# M17-vertical-pack-ap-specialist - Track M (M-Tmpl), Phase 3, Tier 2

**Owner**: claude. **Depends on**: M15 MERGED; M2 MERGED (doc extraction).
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M17-vertical-pack-ap-specialist`.

## Scope

The AP Specialist vertical pack. 5 skills: process-invoice, match-po, flag-exception, route-for-approval, update-erp. Covers invoice ingest -> extraction -> 3-way match -> exception routing -> ERP update. Tools: M365/GWorkspace email, M-Doc extraction, NetSuite/SAP/Sage MCP connectors.

LOC: ~2000. Branch: `claude-M17-vertical-pack-ap-specialist`. PR title: `feat(packs): AP Specialist vertical pack (M17)`.

## Files you may touch

- NEW packs/ap-specialist/ directory (same shape as M16)
- - pack.toml, worker.toml (default_display_name=AP Specialist)
- - 5 skill IR files
- - memory/seed.json (3-way match rules)
- - evals/*.json (>=25 fixtures)
- - README.md

## Files you must NOT touch

- M15 loader internals; M2 extraction internals.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Pack installs cleanly.
- [ ] The AP Specialist pack runs end-to-end on 5 sample invoices (varied formats).
- [ ] 3-way match flags one deliberate mismatch.
- [ ] Eval suite green.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M17-vertical-pack-ap-specialist`
- PR title: `feat(packs): AP Specialist vertical pack (M17)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 1 + 3.
