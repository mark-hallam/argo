# mission-vertical-marketing-intern — Marketing Intern vertical (content + analytics + scheduling)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-marketing-intern`.

## Scope

v2.2 demo vertical. Marketing Intern: content drafting (blog/social), analytics reporting (UTM/funnel), scheduling (calendar), competitive monitoring.

LOC: ~1000.

## Files you may touch

- NEW `packs/marketing-intern/pack.toml` + skills + evals + memory.
- NEW `packs/marketing-intern/skills/{draft_blog,draft_social,analytics_report,schedule_post,competitive_monitor}.skill.json` — 5+ skill IRs.
- NEW `packs/marketing-intern/evals/*.json` — 8+ fixtures.
- NEW `packs/marketing-intern/README.md`.

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

- Branch: `codex-mission-vertical-marketing-intern`
- PR title: `feat: Marketing Intern vertical (content + analytics + scheduling) (mission-vertical-marketing-intern)`

## Notes for the implementer

- Priority: v2.2 demo signal.
- Authored as part of post-v1 backlog batch 2026-05-15.
