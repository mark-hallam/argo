# web-rename-duty-to-schedule-in-ui-copy

**Owner**: codex.
**Depends on**: nothing (mechanical rename in `web/`).
**Authored against**: origin/main `bdf57f458` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-rename-duty-to-schedule-in-ui-copy`.

🟡 **P2 — operator-locked vocabulary: 'Duty' is internal-only, 'Schedule' is operator-facing.** Source: operator memory `project_vocabulary_duty_vs_schedule` (locked 2026-05-23) + evaluator round-16 SHARED.md handoff.

> "In any SPA / hire wizard / marketing / docs / settings copy, use 'Schedule' not 'Duty'. Code/SQL/audit/brief filenames keep 'duty' (no churn). Internal API path /api/v1/workers/<id>/duties/ stays — internal contract."

Today the SPA exposes the engineering word "Duty" / "Duties" in operator-visible labels and headings. This brief renames all such copy to "Schedule" / "Schedules". Codebase identifiers, SQL columns, audit-row kinds, brief filenames, and internal API paths are LEFT UNCHANGED.

## Scope

Sweep `web/` for operator-facing strings containing "Duty" / "Duties" / "duty" / "duties" (as words, not substrings) and rename to "Schedule" / "Schedules" / "schedule" / "schedules" where the string is operator-visible (`<h*>`, `<p>`, `<label>`, `<button>`, `<title>`, alt text, copy in tooltips, copy in toast messages, copy in JSON used as labels).

LEAVE UNTOUCHED:
- Identifiers — `dutyId`, `getDuty`, `Duty` type/interface names — STAY.
- Imports of types/components named `Duty*` — STAY.
- API paths — `/api/v1/workers/[id]/duties` — STAY.
- Internal data attributes / test-ids — STAY (used by Playwright + grep-based debug tooling).
- Comments — STAY.
- Storybook story IDs — STAY (anchors).

## Files you may touch

- Any `*.tsx` / `*.ts` / `*.mdx` under `web/app/`, `web/components/`, `web/lib/`, `web/content/` where the change is to an operator-visible string.
- Storybook story `.mdx` operator-visible copy.

## Files you must NOT touch

- `crates/**` (Rust source).
- `docs/build-plan/streams/**` (brief filenames carry "duty" — they STAY).
- `docs/skycot/operator/**` (per operator vocab memory, the operator runbook MAY use either term — defer to a separate brief; not in scope here).
- API client code that calls `/api/v1/.../duties` — STAY.
- Database / migration / SQLx files.

## Acceptance criteria

- [ ] `rg -i '\bdut(y|ies)\b' web/app web/components web/content` returns ONLY: identifiers, test-ids, comments, API-path strings.
- [ ] Playwright assertion: a new test `web/tests/no-duty-copy.spec.ts` walks the 8 in-product pages (settings/workflows/cohorts/workers/audit/approvals/skills/hire) and asserts the operator-visible text content contains NO "Duty" or "Duties" (case-insensitive word match on `document.body.innerText`).
- [ ] At least 3 SPA pages (settings/workflows/cohorts) include a corresponding "Schedule" / "Schedules" string where they previously said "Duty" / "Duties" (positive test in the same spec).
- [ ] `pnpm typecheck` + `pnpm lint` clean in `web/`.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- Type/lint: clean across `web/`.
- Unit/Playwright: the new `no-duty-copy.spec.ts` is the contract.
- Manual: operator-facing screenshot in PR body showing one renamed page.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Renaming `crates/**` types or SQL columns (operator-locked: code stays "duty").
- Renaming the `/api/v1/workers/[id]/duties` HTTP path (internal contract per operator).
- Renaming brief filenames or any `docs/**` references.

~100-200 LOC (mostly small string edits across ~10-20 files plus the new Playwright spec). Fast-mode-eligible mechanical work — codex preferred.
