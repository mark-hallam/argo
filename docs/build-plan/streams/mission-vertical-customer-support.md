# mission-vertical-customer-support — Customer Support Agent vertical (full Tier-1 + Tier-2 skills)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-customer-support`.

## Scope

v2.2 demo vertical. Full Customer Support agent: ticket triage, intent classification, knowledge-base lookup, response drafting, escalation routing. Builds on existing support-tier1 pack (M19); this extends to Tier-2 (issue research, refund processing, account changes).

LOC: ~1200.

## Files you may touch

- NEW `packs/customer-support/pack.toml` + skills + evals + memory.
- NEW `packs/customer-support/skills/{triage,kb_lookup,respond,escalate,refund,account_change}.skill.json` — 6+ skill IRs.
- NEW `packs/customer-support/evals/*.json` — 10+ fixtures.
- NEW `packs/customer-support/README.md`.

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

- Branch: `codex-mission-vertical-customer-support`
- PR title: `feat: Customer Support Agent vertical (full Tier-1 + Tier-2 skills) (mission-vertical-customer-support)`

## Notes for the implementer

- Priority: v2.2 demo signal.
- Authored as part of post-v1 backlog batch 2026-05-15.
