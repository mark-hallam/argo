# M20-vertical-pack-deal-desk - Track M (M-Tmpl), Phase 3, Tier 3

**Owner**: claude. **Depends on**: M15 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M20-vertical-pack-deal-desk`.

## Scope

The Deal Desk Analyst vertical pack. 5 skills: validate-quote, check-discount-policy, flag-non-standard-terms, route-deal-desk-approval, draft-deal-summary. Default_display_name=Deal Desk Analyst.

LOC: ~1800. Branch: `claude-M20-vertical-pack-deal-desk`. PR title: `feat(packs): Deal Desk Analyst vertical pack (M20)`.

## Files you may touch

- NEW packs/deal-desk/ directory
- - pack.toml, worker.toml
- - 5 skill IR files
- - memory/seed.json (discount thresholds, term policies)
- - evals/*.json (>=20 fixtures)
- - README.md

## Files you must NOT touch

- M15 loader internals.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Pack installs cleanly.
- [ ] End-to-end demo: incoming quote -> validate -> flag non-standard terms -> route to deal-desk approver.
- [ ] Eval suite green.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M20-vertical-pack-deal-desk`
- PR title: `feat(packs): Deal Desk Analyst vertical pack (M20)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 1 + 3.
