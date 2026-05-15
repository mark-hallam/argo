# mission-vertical-sales-sdr — Sales SDR vertical (full prospecting + outreach + qualification)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-sales-sdr`.

## Scope

v2.2 demo vertical. Sales Development Rep: lead enrichment, personalized outreach drafting, response classification, qualification scoring, CRM logging. Builds on existing salesforce + zendesk connectors.

LOC: ~1100.

## Files you may touch

- NEW `packs/sales-sdr/pack.toml` + skills + evals + memory.
- NEW `packs/sales-sdr/skills/{enrich,draft_outreach,classify_reply,qualify,log_to_crm}.skill.json` — 5+ skill IRs.
- NEW `packs/sales-sdr/evals/*.json` — 10+ fixtures.
- NEW `packs/sales-sdr/README.md`.

## Files you must NOT touch

- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Implementation matches scope above.
- [ ] All new tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-vertical-sales-sdr`
- PR title: `feat: Sales SDR vertical (full prospecting + outreach + qualification) (mission-vertical-sales-sdr)`

## Notes for the implementer

- Priority: v2.2 demo signal.
- Authored as part of post-v1 backlog batch 2026-05-15.
