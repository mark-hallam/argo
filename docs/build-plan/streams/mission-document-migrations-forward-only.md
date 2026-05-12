# Mission load-bearing wire — Document migrations as forward-only

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `2ca2d694` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #267. We've shipped 61 migration files (v1 … v61) and the operator-facing model is forward-only — there is no rollback path. This invariant is implicit in the migration runner today; make it explicit in the doc layer so operators don't ask "can I downgrade?" with no answer. Doc-only brief; no code touched. **Tier-Doc.**

## Scope

Single-purpose doc-only brief:

1. **NEW** `docs/skycot/operator/migrations.md` — operator-facing reference:
   - Migration model: each `vN.rs` is applied exactly once, in order, on startup. No down-migrations.
   - Why forward-only: simpler reasoning about prod state; SQLite's online-schema-change limitations; dataset isn't large enough to justify Liquibase-style two-way migrations.
   - What to do if a migration is buggy: forward-fix via `vN+1` rather than rollback `vN`. Document the canonical fix-forward pattern.
   - Compatibility rules: additive only (new columns NULL-able OR with default; new tables fine; new indexes fine). Non-additive (DROP COLUMN, type change, rename) requires a 3-step dance: add-new-column + dual-write + remove-old-column across at least 2 releases.
   - Disaster recovery: backup-restore *before* the bad migration is the only "rollback" supported.
   - Cross-reference: `crates/argo-store/src/migrations/mod.rs` for the runner + the version table.

2. **EDIT** `docs/skycot/RUNBOOK.md` — add a "Migrations" subsection that points at the new doc and notes the forward-only rule.

3. **EDIT** `docs/dev-runtime-guardrails.md` — add a top banner clarifying this is DEV ONLY (per backlog #271's framing — small bonus, same theme): the file documents dev-mode loopholes that production never sees. Operator should not consult it for prod posture.

4. **EDIT** `CLAUDE.md` — under "Maintain" the rule `additive migrations` already exists; add a one-line reference to the new doc for the rationale.

NOT in scope: writing the migration runner; refactoring the migrations layout; adding migration linting (separate brief if wanted); auditing existing migrations for non-additive offences.

## Files you may touch

- `docs/skycot/operator/migrations.md` (NEW, ≤ 350 LOC)
- `docs/skycot/RUNBOOK.md` — append 1 subsection (≤ 20 LOC additive)
- `docs/dev-runtime-guardrails.md` — prepend a `> DEV ONLY` banner (≤ 8 LOC)
- `CLAUDE.md` — extend the `additive migrations` bullet with `(see docs/skycot/operator/migrations.md)` (1 line)

## Files you must NOT touch

- `crates/argo-store/src/migrations/` — doc-only brief, zero code edits
- Any `v*.rs` migration file
- Any test
- `Cargo.toml`

## Acceptance criteria

- [ ] `docs/skycot/operator/migrations.md` exists with the 6 sections above
- [ ] RUNBOOK cross-link added
- [ ] dev-runtime-guardrails.md prefixed with the DEV ONLY banner
- [ ] CLAUDE.md cross-link added
- [ ] No code changes (verify via `git diff --stat -- 'crates/'` empty)
- [ ] BUILD_STATUS.md updated; all touched files < 500 LOC

## Branch + PR

- Branch: `claude-mission-document-migrations-forward-only`
- PR title: `docs: forward-only migrations + DEV-ONLY guardrails banner (mission-document-migrations-forward-only)`

## Notes

- Keep the new doc operator-facing, not implementer-facing. "What an operator needs to know when they see a migration land in prod" — not "how to write a migration." (Implementer rules can live in CONTRIBUTING_AGENT.md if missing; out of scope here.)
- The DEV ONLY banner on `dev-runtime-guardrails.md` is preventative: search history shows the doc was once linked from an operator runbook by mistake. Reduce the risk that a fresh agent or operator reaches that page and treats its content as production posture.
- This brief is intentionally small (~400 LOC of pure prose). Worker should be able to land it in one sitting.
