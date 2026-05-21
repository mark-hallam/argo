# codex-prepared-statement-coverage-audit

**Authored against**: `origin/main` `bf8c69f82` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-prepared-statement-coverage-audit`.

**Fast-mode eligible**: yes — mechanical audit.

## Summary

Project invariant (per CLAUDE.md "Maintain" section): SQL queries should use prepared statements (parameterized queries via `rusqlite::params!` / `sqlx::query!`), not string-formatted SQL. The existing `prepared-statements` CI gate (per `.github/workflows/DISABLED_WORKFLOWS_POLICY.md`) was operator-disabled. Codex re-runs the audit locally to confirm zero violations.

## Acceptance

- [ ] Audit script (or `rg` invocation) enumerates every `format!`, `to_string()`, or string-concat that produces SQL. Standard pattern to grep: `format!.*SELECT|format!.*INSERT|format!.*UPDATE|format!.*DELETE` plus `\\$\\{[a-z_]+\\}` injection patterns inside SQL strings.
- [ ] PR body includes table: file:line | pattern | classification.
- [ ] Each finding classified:
  - **PARAMETERIZED-OK** — uses `params!` / bind variables correctly.
  - **TABLE-NAME-LITERAL-OK** — concatenates a table name (which can't be a bind variable in SQLite) but the value is a hardcoded constant. Safe.
  - **INJECTION-RISK** — user-controlled value concatenated into SQL string. Real bug.
- [ ] For each INJECTION-RISK: fix in this brief (rewrite to `params!`).
- [ ] If `scripts/check-prepared-statements.sh` (or similar) exists, run it and include output in PR body.
- [ ] No findings → close as RESOLVED-NO-FINDINGS.

## May touch

- Crate files where an INJECTION-RISK swap is the fix.
- Test files only if adding a negative test (SQL injection attempt → rejected).

## Must NOT touch

- The `prepared-statements` CI workflow (operator-disabled per policy).
- Migration files (those run once at startup; not request-path SQL).
- Schema definitions.

## Notes

- Pre-flight: `cat scripts/check-prepared-statements.sh` if it exists; reuse the canonical script if so.
- Sizing: 100–300 LOC. Mostly findings table + ≤2 surgical fixes if any.
- v1.0 launch hygiene — SQL injection is the bug class that fails procurement audits the loudest.
