# fix-workflow-detail-bun-history-typo

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workflow-detail-bun-history-typo`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — Workflow detail page run-history section header reads "BUN HISTORY · LAST 7 DAYS" — single-character typo. Visible in Scene 5 if the camera holds on the detail view.**

## Root cause

Auditor saw:

> The history strip on `/workflows/marketplace/customer_research_followup` shows the section header as "BUN HISTORY · LAST 7 DAYS" — looks like a single-char typo of "RUN". Visible to the camera if Scene 5 holds on the detail view.
> — `whole-site-mvp-claims-coverage.md` F09

Likely in one of:
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`
- `web/components/skycot/workflow/RunsPanel.tsx`
- A locked design memo / Storybook fixture

`rg "BUN HISTORY|Bun history|BUN_HISTORY" web/` will locate it.

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F09 [P2] — workflow detail "BUN HISTORY" typo (should be "RUN HISTORY")

## Fix recipe

1. Grep for the literal string `BUN HISTORY` in `web/` and any locked-design markdown under `docs/skycot/design-explorations/`.
2. Replace with `RUN HISTORY` (preserving case).
3. If the string came from a design exploration HTML mockup, fix that too so the next reference render lands correctly.
4. Smoke-check the workflow detail page renders the corrected header.
5. Vitest snapshot, if any, will need re-baseline.

## Acceptance criteria

- [ ] No remaining `BUN HISTORY` literals anywhere in `web/`.
- [ ] `/workflows/marketplace/customer_research_followup` renders "RUN HISTORY · LAST 7 DAYS".
- [ ] Vitest snapshot tests pass.
- [ ] PR body cites finding F09.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`
- `web/components/skycot/workflow/RunsPanel.tsx`
- Any design-exploration HTML in `docs/skycot/design-explorations/` containing the typo

## Out of scope

- Fabricated run-history data (covered by `fix-workflow-run-history-fabricated-rows-in-mock-offline`)
- Run-now button label (covered by `fix-workflow-run-button-label-alignment`)

## Effort

XS — ~1-line change. ~10 minutes including PR.
