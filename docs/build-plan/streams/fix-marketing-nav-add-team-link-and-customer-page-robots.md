# fix-marketing-nav-add-team-link-and-customer-page-robots

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-nav-add-team-link-and-customer-page-robots`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F11 [P1] + F21 [P2].

🟡 **P2 — `/team` is in the in-app sidebar but unreachable from any marketing page (landing / pricing / customers / compare). Investors landing on `/landing` cannot see the team-roster value-prop without signing in. Separately, the 5 fabricated customer detail pages are indexable.**

## Root cause

- F11: `grep -o 'href="/team"' /tmp/landing.html /tmp/pricing.html /tmp/customers.html /tmp/compare.html` returns zero. MarketingNav does not list /team.
- F21: `robots.txt: Allow: /customers` AND `/customers` links to 5 fabricated detail pages.

## Findings this brief closes

- whole-site-mvp-claims-coverage F11 [P1] (downgraded to P2 — not a POC blocker, public-release prep) — `/team` not linked from marketing
- whole-site-mvp-claims-coverage F21 [P2] — fabricated customer pages indexable (POC OK, public-release blocker)

## Fix recipe

1. **Add `/team` to MarketingNav** in `web/components/skycot/marketing/MarketingNav.tsx`. Label "Roster" or "Meet the team." Reachable from any marketing page.
   - Alternative: build a marketing-mirror at `/teammates` that renders a public-friendly subset of /team (no in-app chrome).
2. **For F21 (public-release prep, not v1.0 POC blocker)**:
   - Add `Disallow: /customers/*` to `web/public/robots.txt` until customers are real.
   - Add `noindex` meta to each `/customers/<slug>` page until then.
   - Keep `/customers` (the index page with logo grid) indexable so SEO has the gallery; only block the detail pages.
3. **Web test**: assert `/team` link present in MarketingNav rendered HTML.

## Acceptance criteria

- [ ] MarketingNav contains a link to `/team` (or a public mirror surface).
- [ ] `robots.txt` disallows `/customers/*` detail pages.
- [ ] `/customers/<slug>` pages carry `noindex` meta.
- [ ] Web test asserts the new link.

## May-touch

- `web/components/skycot/marketing/MarketingNav.tsx` (add link)
- `web/public/robots.txt` (disallow detail)
- `web/app/customers/[slug]/page.tsx` (noindex meta)
- `web/tests/marketing-nav-includes-team.test.ts` (NEW)

## Out of scope

- Building a public `/teammates` mirror page (deferred unless operator wants it)
- Real customer logos / case studies (post-v1.0 marketing work)

## Effort

~30-50 LOC + 1 test. ~45min.
