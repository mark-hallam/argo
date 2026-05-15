# mission-vertical-finance-ar — Accounts Receivable vertical (invoice + collection + reconciliation)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-finance-ar`.

## Scope

v2.2 demo vertical. Accounts Receivable: invoice creation, dunning emails on overdue, payment reconciliation, customer aging report. Builds on existing AP Specialist (M17) + Stripe connector.

LOC: ~1100.

## Files you may touch

- NEW `packs/finance-ar/pack.toml` + skills + evals + memory.
- NEW `packs/finance-ar/skills/{create_invoice,dunning_email,reconcile_payment,aging_report}.skill.json` — 4+ skill IRs.
- NEW `packs/finance-ar/evals/*.json` — 10+ fixtures.
- NEW `packs/finance-ar/README.md`.

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

- Branch: `codex-mission-vertical-finance-ar`
- PR title: `feat: Accounts Receivable vertical (invoice + collection + reconciliation) (mission-vertical-finance-ar)`

## Notes for the implementer

- Priority: v2.2 demo signal.
- Authored as part of post-v1 backlog batch 2026-05-15.
