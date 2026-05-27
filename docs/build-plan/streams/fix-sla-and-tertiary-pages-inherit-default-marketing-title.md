# fix-sla-and-tertiary-pages-inherit-default-marketing-title

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-sla-and-tertiary-pages-inherit-default-marketing-title`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F05)

🟢 **P3 — `/sla` renders `document.title = "SKYCOT — Hire your first AI teammate"` (the layout-level title) instead of `"SLA · SKYCOT"`. Compare with `/pricing` ("Pricing · SKYCOT"), `/security` ("Security · SKYCOT"), `/customers` ("Customers · SKYCOT") which all carry page-specific titles. Missing `export const metadata = { title: "SLA" }` in the sla layout. Polish nit but adds up across tertiary pages.**

## Root cause

`web/app/sla/layout.tsx` (or `web/app/sla/page.tsx` if no layout file) doesn't export a `metadata` object with a `title`. Next.js falls through to the root marketing layout's default title `"SKYCOT — Hire your first AI teammate"`. The well-behaving pages (`/pricing`, `/security`, `/customers`) all carry `export const metadata = { title: "..." }`.

Also worth checking in the same pass:

- `/help` — verify the title.
- `/changelog/rss.xml` — likely intentional but worth confirming.
- `/for/[slug]` — vertical landing pages; check the title template covers them.

Sibling `fix-spa-page-titles-metadata-leak-default-approvals` (status not confirmed) and `fix-onboarding-step-pages-tab-title-via-layout-metadata` (TODO) covered similar drift on /approvals and onboarding step pages. This brief is the marketing/tertiary-page complement.

## Findings this brief closes

- whole-site F05 [P3] — `/sla` (and likely a few siblings) inherit the default marketing title instead of their own

## Fix recipe

1. **Audit every page under `web/app/`** for missing `export const metadata`:
   ```
   for f in web/app/**/page.tsx; do
     if ! grep -q "export const metadata" "$f"; then echo "$f"; fi
   done
   ```
   Triage the result — keep pages that intentionally inherit; fix the rest.
2. **Add page-specific `metadata` exports** to each affected file. For SLA:
   ```ts
   // web/app/sla/layout.tsx or page.tsx
   export const metadata = {
     title: "SLA · SKYCOT",
     description: "Service-level agreement for Skycot enterprise customers.",
   };
   ```
3. **Verify the rendered tab title** at runtime for each fixed page by curling and checking `<title>`:
   ```
   curl -s http://localhost:3000/sla | grep "<title>"
   ```
4. **Cover with a smoke test** at `web/tests/page-title-coverage.test.ts` (NEW) that walks every page route and asserts the rendered `<title>` is not the default marketing title (heuristic: doesn't equal the root `metadata.title`).

## Acceptance criteria

- [ ] `/sla` renders `<title>SLA · SKYCOT</title>`.
- [ ] All tertiary pages identified in the audit have a page-specific `metadata.title`.
- [ ] No regression on pages with correct titles already.
- [ ] Page-title coverage test passes.
- [ ] PR body cites whole-site F05 and lists every page that was fixed.

## May-touch

- `web/app/sla/layout.tsx` or `web/app/sla/page.tsx`
- `web/app/help/layout.tsx` (if affected)
- `web/app/for/[slug]/layout.tsx` (if affected)
- `web/app/changelog/layout.tsx` (likely OK; verify)
- `web/tests/page-title-coverage.test.ts` (NEW)

## Out of scope

- /approvals title leak (covered by `fix-spa-page-titles-metadata-leak-default-approvals`)
- Onboarding step page titles (covered by `fix-onboarding-step-pages-tab-title-via-layout-metadata`)
- Worker detail page title (covered by `fix-worker-detail-page-title-and-server-client-tab-helper` BLOCKED)
- Settings sub-page h1 (covered by separate brief on whole-site F14)

## Effort

XS — ~5 LOC per affected file + ~40 LOC coverage test. ~1-2 hours including the audit pass.
