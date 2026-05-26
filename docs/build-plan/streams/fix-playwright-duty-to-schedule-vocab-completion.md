# fix-playwright-duty-to-schedule-vocab-completion

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-playwright-duty-to-schedule-vocab-completion`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` Playwright failure cluster: Vocabulary Duty → Schedule (6 specs).

🟠 **P1 — Six `no-duty-copy` Playwright specs fail because the settings, workflows, and cohorts pages still say "Duty" and do not yet say "Schedule". Per operator-locked memory `project_vocabulary_duty_vs_schedule` (2026-05-23), user-facing UI must use "Schedule" not "Duty" — code/SQL/audit/brief filenames keep "duty". This is mechanical rollout work on three remaining pages.**

## Root cause

Memory `project_vocabulary_duty_vs_schedule` (operator-locked 2026-05-23):
> "in any SPA / hire wizard / marketing / docs / settings copy, use 'Schedule' not 'Duty'; code/SQL/audit/brief filenames keep 'duty' (no churn). Briefs in flight: web-rename-duty-to-schedule-in-ui-copy + web-workflow-detail-add-as-schedule-button."

The 6 failing specs (per the playwright run.log under this audit directory) assert that the words "Duty" / "Duties" do NOT appear in user-visible copy on `/settings`, `/workflows`, `/cohorts` pages, AND that "Schedule" / "Schedules" DO appear in their place. The prior `web-rename-duty-to-schedule-in-ui-copy` brief landed partial coverage (worker overview, hire wizard) but did not sweep settings / workflows / cohorts.

## Findings this brief closes

- chrome-live-walk-and-playwright Playwright cluster: Vocabulary Duty → Schedule (6 specs failing)
  - `no-duty-copy: /settings` × 2 (does say "Duty", does NOT say "Schedule")
  - `no-duty-copy: /workflows` × 2 (same)
  - `no-duty-copy: /cohorts` × 2 (same)

## Fix recipe

1. **Read the playwright run.log** for the exact selectors and expected strings per failing spec.

2. **Sweep `web/app/(app)/settings/**/*.tsx` + `web/components/skycot/settings/**/*.tsx`** for user-visible "Duty"/"Duties" strings. Replace with "Schedule"/"Schedules". Examples likely to need attention:
   - Schedule preferences pages
   - Hours / availability copy
   - Cron expression labels

3. **Sweep `web/app/(app)/workflows/**/*.tsx` + `web/components/skycot/workflows/**/*.tsx`** for "Duty"/"Duties" in cards, headings, modal copy. Specifically:
   - Workflow templates page (50 templates may use "Duty" in titles)
   - Workflow detail page
   - Workflow card metadata

4. **Sweep `web/app/(app)/cohorts/**/*.tsx` + `web/components/skycot/cohorts/**/*.tsx`** for "Duty"/"Duties" in dept tab labels, persona cards, KPI copy.

5. **Keep all non-user-facing strings as `duty`**: testid attributes (`data-testid="duty-toggle-d-list"`), localStorage keys, audit kind strings, internal types/enums, brief filenames, file names. Per memory, "no churn" on code-side identifiers.

6. **Add a single comprehensive Playwright spec** at `web/playwright/no-duty-copy-comprehensive.spec.ts` that asserts across 8+ in-app routes (settings root + sub-tabs + workflows + cohorts + workers + audit + approvals) that user-visible text does NOT contain "Duty"/"Duties" and DOES contain "Schedule"/"Schedules" where the concept applies. Replaces the 6 fragmented specs.

7. **Update related copy** if any error messages, empty states, or tooltips reference "duty" as user-facing text.

## Acceptance criteria

- [ ] `pnpm playwright test "no-duty-copy"` passes 0 failed
- [ ] `grep -ri ">Duty<\|>Duties<" web/app web/components` excluding test files + testid attributes returns 0 hits
- [ ] All testid attributes still use `duty-*` (no code churn beyond user-visible strings)
- [ ] All audit event kinds and gateway routes still use `duty` (no backend churn)

## May-touch

- `web/app/(app)/settings/**/*.tsx`
- `web/app/(app)/workflows/**/*.tsx` (excluding builder)
- `web/app/(app)/cohorts/**/*.tsx`
- `web/components/skycot/{settings,workflows,cohorts}/**/*.tsx`
- `web/playwright/no-duty-copy-comprehensive.spec.ts` (NEW; replaces fragmented specs)
- `web/playwright/no-duty-copy-*.spec.ts` (OLD; delete after the comprehensive spec covers)

## Out of scope

- The "duty" → "schedule" rename in worker overview (already shipped via prior brief)
- The "Run as schedule" button on workflow detail (covered by `web-workflow-detail-add-as-schedule-button`)
- Code-side renaming of `duty` types / enums / cron tables / audit kinds (operator-decreed: no churn)

## Effort

~50-150 LOC string edits across 3 page trees + 1 consolidated spec. ~2-4 hours. P1 because vocabulary inconsistency on camera reads as half-finished software.
