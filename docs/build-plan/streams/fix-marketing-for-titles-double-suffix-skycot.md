# fix-marketing-for-titles-double-suffix-skycot

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-for-titles-double-suffix-skycot`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md` F05 + F06 + F07.

🟠 **P1 — `/for/*` vertical landing pages emit double-suffixed browser-tab titles like "AI sales teammates that never sleep — SKYCOT · SKYCOT". `/sla` falls to the site-default title. `/workflows/templates` is missing the global `· SKYCOT` suffix. Visible in the browser tab any time the camera frames the URL bar.**

## Root cause

Three coupled defects in metadata title composition:

1. `/for/*` vertical landing shells (`web/app/for/_components/VerticalLandingShell.tsx` and/or per-vertical `layout.tsx`) set `metadata.title` containing `" — SKYCOT"` in the title string itself, AND the root `web/app/layout.tsx` has a `title.template = "%s · SKYCOT"` that appends another `· SKYCOT`. Net result: "AI sales teammates that never sleep — SKYCOT · SKYCOT".

2. `/sla` (`web/app/sla/page.tsx`) does NOT export any `metadata`, so it inherits the site-default title from the root layout — different format from `/privacy`, `/terms`, `/security` which all have their own metadata.

3. `/workflows/templates` (`web/app/(app)/workflows/templates/page.tsx`) likely uses `title: { absolute: "Workflow templates" }` or `default` which bypasses the template, producing "Workflow templates" without the `· SKYCOT` suffix.

Curl evidence:
```
curl -s http://localhost:62927/for/sales | perl -nle 'print $1 if /<title[^>]*>([^<]+)/'
→ "AI sales teammates that never sleep — SKYCOT · SKYCOT"

curl -s http://localhost:62927/sla | grep '<title>'
→ "SKYCOT — Hire your first AI teammate"   (site-default)

curl -s http://localhost:62927/workflows/templates | perl -nle 'print $1 if /<title[^>]*>([^<]+)/'
→ "Workflow templates"                      (no suffix)
```

## Findings this brief closes

- whole-site-mvp-claims-coverage F05 [P1] — `/for/*` double-suffix `SKYCOT · SKYCOT`
- whole-site-mvp-claims-coverage F06 [P2] — `/sla` lacks page-specific title
- whole-site-mvp-claims-coverage F07 [P2] — `/workflows/templates` missing global `· SKYCOT` suffix

## Fix recipe

1. **Strip the trailing `— SKYCOT` from `/for/*` title strings** in `web/app/for/_components/VerticalLandingShell.tsx` (or per-vertical layout — verify both). Each per-vertical title should be: `"AI <vertical> teammates that never sleep"` — the global template appends `· SKYCOT`.

2. **Add metadata to `/sla`** at `web/app/sla/page.tsx`:
   ```ts
   export const metadata: Metadata = {
     title: "SLA",
     description: "Skycot's service-level agreement.",
   };
   ```
   Will resolve to "SLA · SKYCOT" via the template.

3. **Fix `/workflows/templates` title** at `web/app/(app)/workflows/templates/page.tsx` (or its layout):
   ```ts
   export const metadata: Metadata = {
     title: "Workflow templates",   // NOT `{ absolute: "..." }`
   };
   ```
   Removes any `absolute:` or `default:` override so the template fires.

4. **Add a docs-test or web-test** that asserts every `(app)/*/page.tsx` and `for/*/page.tsx` emits a title matching the pattern `^[^·]+ · SKYCOT$` (single suffix). Pattern: `web/tests/page-titles-suffix.test.ts`. Iterate the route list, fetch each, regex-check the title.

## Acceptance criteria

- [ ] `curl /for/sales` title = `"AI sales teammates that never sleep · SKYCOT"` (single suffix).
- [ ] Same for `/for/recruiting`, `/for/marketing`, `/for/finance`, `/for/ops`, `/for/support`.
- [ ] `curl /sla` title = `"SLA · SKYCOT"`.
- [ ] `curl /workflows/templates` title = `"Workflow templates · SKYCOT"`.
- [ ] No other page regresses (smoke-iterate route list).
- [ ] New title-suffix test covers all `/for/*` + `/sla` + `/workflows/templates`.

## May-touch

- `web/app/for/_components/VerticalLandingShell.tsx` (strip `— SKYCOT`)
- `web/app/for/<vertical>/layout.tsx` × 6 (verify per-vertical metadata if it exists)
- `web/app/sla/page.tsx` (add metadata)
- `web/app/(app)/workflows/templates/page.tsx` (fix metadata.title)
- `web/tests/page-titles-suffix.test.ts` (NEW)

## Out of scope

- The unrelated batch of missing-title pages (`/brand`, `/verticals`, `/orchestration`, `/genesis`) leaking "Approvals · SKYCOT" — covered by `fix-spa-page-titles-metadata-leak-default-approvals`.
- Marketing nav copy

## Effort

~40-80 LOC across 8-9 small files + 1 test. ~1-2 hours.
