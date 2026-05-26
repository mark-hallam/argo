# fix-marketing-about-and-contact-stub-pages

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-about-and-contact-stub-pages`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/whole-site-mvp-claims-coverage.md` F7 [P1].

🟠 **P1 — `/about` and `/contact` both return 404. Standard marketing footer expectations. Neither is linked from `/landing` today, but every other site has them; investors will guess-type them, and the demo URL handed to a buyer will produce 404s on these paths.**

## Root cause

`web/app/` has no `/about` or `/contact` route directories. `/pricing` includes a `mailto:hello@skycot.com` CTA so the contact intent is partially satisfied there — but a direct visit to `/contact` returns the Next.js default 404 page.

Per the function-vs-dressing rule: this is dressing for the POC demo (the camera doesn't visit these), so P1, not P0. But it's a 1-hour fix and a real public-credibility win — operator may share the demo URL externally.

## Findings this brief closes

- whole-site-mvp-claims-coverage F7 [P1] — `/about` and `/contact` 404 but `/landing` doesn't link to them — orphan expectations

## Fix recipe

1. **Build `/about`** at `web/app/about/page.tsx`:
   - Marketing layout (no sidebar).
   - Hero copy: "Skycot — built in Melbourne." Per memory `project_skycot_melbourne_australia`, mention Melbourne (not Sydney).
   - 2-3 paragraphs of brand-restrained editorial copy: what Skycot is, who built it, the mission ("Don't buy another tool. Hire a teammate.").
   - Founder name + photo placeholder (operator pick).
   - CTA at the bottom: "Try a demo workspace" → `/auth/signup`.
   - Set `<title>About · SKYCOT</title>`.

2. **Build `/contact`** at `web/app/contact/page.tsx`:
   - Marketing layout.
   - Hero: "Get in touch."
   - Three options laid out vertically:
     - **Email** — `mailto:hello@skycot.com` (same address `/pricing` uses).
     - **Demo** — link to `/auth/signup` ("Try a demo workspace · no email required" — keeping the copy honest per `fix-signup-no-email-required-button-honest-copy`).
     - **Twitter/X + LinkedIn** — operator's handles.
   - Set `<title>Contact · SKYCOT</title>`.

3. **Add both pages to the marketing footer nav** at `web/components/skycot/marketing/MarketingFooter.tsx` (or wherever the footer is defined). Group with the existing "Privacy / Terms / SLA / Security" cluster.

4. **Update `web/app/sitemap.ts`** to include `/about` and `/contact`.

5. **Add a Playwright spec** `web/playwright/marketing/about-and-contact-resolve.spec.ts`:
   - Visit `/about` → 200, contains `<h1>`, title is "About · SKYCOT".
   - Visit `/contact` → 200, contains `<h1>`, title is "Contact · SKYCOT", contains `mailto:hello@skycot.com`.
   - Footer on `/landing` contains both links.

## Acceptance criteria

- [ ] `/about` returns 200 with brand-restrained editorial copy.
- [ ] `/contact` returns 200 with the 3 contact options.
- [ ] Both pages have correct `<title>` tags ending in `· SKYCOT`.
- [ ] MarketingFooter links to both pages.
- [ ] Sitemap includes both.
- [ ] New Playwright spec passes.

## May-touch

- `web/app/about/page.tsx` (NEW)
- `web/app/contact/page.tsx` (NEW)
- `web/components/skycot/marketing/MarketingFooter.tsx` (links)
- `web/app/sitemap.ts` (include new routes)
- `web/playwright/marketing/about-and-contact-resolve.spec.ts` (NEW)

## Out of scope

- A real contact form / Calendly embed (post-MVP per audit F9).
- Founder bios or team page (not yet in scope).
- About-page version of the mission statement / values — keep this minimal for v1.0.

## Effort

~150-250 LOC across 2 pages + footer + spec. ~3-5 hours. P1 — public-credibility fix, not a demo blocker.
