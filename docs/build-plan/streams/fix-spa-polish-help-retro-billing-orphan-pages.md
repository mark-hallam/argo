# fix-spa-polish-help-retro-billing-orphan-pages

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-polish-help-retro-billing-orphan-pages`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md` F08 + F18.

🟢 **P3 — Three orphan routes pollute the SPA: `/help` is a "we're building" placeholder, `/retro` and `/billing` return 200 but aren't in any sidebar nav. Polish miss + crawler/bookmark trap.**

## Root cause

Three independent orphan-route patterns:

1. **`/help`** — `web/app/help/page.tsx` renders only "We're building the in-product help center alongside v1.1." with three escape hatches (changelog, support email, docs). Acceptable for a POC but the marketing nav `web/components/marketing/MarketingNav.tsx:34-41` links there.

2. **`/retro`** — exists at `web/app/(app)/retro/page.tsx`, returns 200, but no nav link points to it. Likely a leftover from an earlier sprint.

3. **`/billing`** — exists at `web/app/(app)/billing/page.tsx`, returns 200, but `/settings/billing` is the canonically-linked billing page. `/billing` is an orphan duplicate.

Any of the three is a discoverability trap: a search engine indexes them, an investor stumbles in via a typed URL, the page reads as half-built.

## Findings this brief closes

- whole-site-mvp-claims-coverage F08 [P2, downgraded to P3] — `/help` is a placeholder
- whole-site-mvp-claims-coverage F18 [P3] — `/retro` and `/billing` orphans

## Fix recipe

1. **`/help` — redirect to `/docs` until the help center ships**:
   - Replace `web/app/help/page.tsx` body with a Next.js `redirect("/docs")` server-component.
   - Update `web/components/marketing/MarketingNav.tsx:34-41` to point at `/docs` directly (drop the `/help` link).
   - When the real help center ships (v1.1), restore the page + nav link.

2. **`/retro` — remove or document**:
   - First check `git log -- web/app/(app)/retro/` to understand why it exists. If it's a leftover, delete the route directory.
   - If it's intentionally hidden but referenced from somewhere (search `rg '"\/retro"' web/`), document the intent in a top-of-file comment.

3. **`/billing` — redirect to `/settings/billing`**:
   - Replace `web/app/(app)/billing/page.tsx` body with `redirect("/settings/billing")`.
   - Robots header: `noindex` so crawlers don't index the orphan.

4. **Robots / sitemap audit**: ensure none of `/help`, `/retro`, `/billing` are listed in `web/app/sitemap.ts`. Drop if present.

5. **Test** `web/tests/orphan-route-redirects.test.ts`:
   - `/help` → 307 to `/docs`.
   - `/billing` → 307 to `/settings/billing`.
   - `/retro` → 200 with documented purpose, OR 404 after deletion (verify decision).

## Acceptance criteria

- [ ] `/help` redirects to `/docs` (or, if operator decides to keep the placeholder, has a clearer "Coming in v1.1" banner with no "help" verb).
- [ ] Marketing nav points at `/docs` directly.
- [ ] `/billing` redirects to `/settings/billing`.
- [ ] `/retro` is either documented inline or removed (operator decision).
- [ ] Sitemap.ts does not list any of the three orphans.
- [ ] Tests cover the redirects.

## May-touch

- `web/app/help/page.tsx` (redirect or rewrite)
- `web/components/marketing/MarketingNav.tsx` (update href)
- `web/app/(app)/billing/page.tsx` (redirect)
- `web/app/(app)/retro/page.tsx` (delete or document)
- `web/app/sitemap.ts` (drop orphans)
- `web/tests/orphan-route-redirects.test.ts` (NEW)

## Out of scope

- The actual help-center implementation
- The `/settings/billing` page itself (already exists)
- Cleanup of the `/maintenance` and `/offline` placeholders (status pages — intentionally minimal)

## Effort

~40-70 LOC across 4 small file edits + 1 test. ~1-1.5 hours.
