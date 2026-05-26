# fix-cohorts-built-in-melbourne-eyebrow-misplaced

**Owner**: codex or claude.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cohorts-built-in-melbourne-eyebrow-misplaced`.
**Source**: ux-e2e-audit-T2309MEL `whole-site-mvp-claims-coverage.md` F15 [P2].

🟢 **P2 — `/cohorts` renders "Other · BUILT IN MELBOURNE" as the eyebrow text on the cohort department block. Reads as decorative metadata attached to a real category — confusing. The "BUILT IN MELBOURNE" flourish belongs in the footer (per `project_skycot_melbourne_australia`), not as an eyebrow on a functional department label.**

## Root cause

The cohort page hero or department-block component is mounting the "BUILT IN MELBOURNE" string as an eyebrow above the "Other · 1 cohort" department label. Either:

- A brand-loud `<EyebrowLabel>` component is being used in the wrong context (eyebrow meant for hero / marketing surfaces, not in-app data labels), OR
- A hardcoded string was added during the brand-loud-D3 consistency pass (memory `project_brand_consistency_pass`) and accidentally targets the cohort tab section

Either way, the camera reads "OTHER · BUILT IN MELBOURNE" as if Melbourne is a sub-category of cohort department, which is not the intent.

## Findings this brief closes

- whole-site-mvp-claims-coverage F15 [P2] — "Other · BUILT IN MELBOURNE" eyebrow appears on the cohort department block

## Fix recipe

1. **Locate the source.**
   ```bash
   rg "BUILT IN MELBOURNE" web/app/(app)/cohorts/
   rg "BUILT IN MELBOURNE" web/components/skycot/cohorts/
   ```

2. **Decide on placement** — two reasonable options:
   - **Option A (preferred — remove from cohorts entirely)**: the cohorts page should NOT carry a "BUILT IN MELBOURNE" eyebrow at all. It's not a marketing surface; it's an in-product data view. Delete the eyebrow render.
   - **Option B (move to footer)**: if the operator wants the Melbourne flourish present on every in-product page, move it to the `AppShell` footer where it appears once globally, not per-page.

3. **Cross-check sibling pages.** Run `rg "BUILT IN MELBOURNE" web/` and verify the eyebrow doesn't appear on any other in-product page (Approvals, Audit, Skills, Workers, Workflows, Settings). If it does, treat as the same defect class.

4. **Cohorts hero recovery.** If the eyebrow was load-bearing for the cohort page's hero design (per `project_cohorts_page_design_locked` D3 magazine-tiles spec), replace with a meaningful in-product label — e.g. `"WORKFORCE BY DEPARTMENT"` or just remove the eyebrow slot.

## Acceptance criteria

- [ ] `/cohorts` no longer renders "BUILT IN MELBOURNE" as an eyebrow on the department block
- [ ] If Option B chosen: the string appears once in the global footer instead
- [ ] No other in-product page (audit, approvals, audit, skills, workers, workflows, settings) carries the eyebrow inappropriately
- [ ] Marketing pages (`/landing`, `/about`, etc.) still keep their Melbourne references unchanged
- [ ] `pnpm --dir web exec vitest run app/(app)/cohorts` green

## May-touch

- `web/app/(app)/cohorts/page.tsx` (~5 LOC remove / replace eyebrow)
- `web/components/skycot/cohorts/CohortsHero.tsx` (or wherever the eyebrow is composed) (~5 LOC)
- `web/components/skycot/AppShell.tsx` footer (only if Option B — ~10 LOC)

## Out of scope

- Marketing page Melbourne mentions (correct + intentional per `project_skycot_melbourne_australia`)
- D3 brand-consistency cohorts work (`fix-cohorts-empty-state-render-locked-d3-magazine-preview`)
- 6-department-tab work (`fix-cohorts-dept-registry-design-lock-drift-6-vs-8`)

## Effort

XS. ~5-15 LOC. ~15 min. P2 minor brand polish — only affects the camera if Scene 1/2 detours into /cohorts (which the runbook says it shouldn't, but unexpected demos might).
