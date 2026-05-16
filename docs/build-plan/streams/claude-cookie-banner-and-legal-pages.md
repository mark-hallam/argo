# claude-cookie-banner-and-legal-pages — GDPR/CCPA banner + privacy + ToS (Wave 4 P1)

**Owner**: claude. **Depends on**: `claude-marketing-site-skycot-com` MERGED (IN_REVIEW #1754).
**Authored against**: origin/main `054d8b0a` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-cookie-banner-and-legal-pages`.
**Source**: §4 weekend plan Wave 4 — required for EU + California traffic.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — EU + California regulatory floor before public launch.
- **Security** (#4) — explicit consent gates analytics/marketing cookies.

## Scope

P1. Three regulatory + trust surfaces on the marketing site (`skycot.com`):

1. **Cookie consent banner.**
   - First-visit dismissable banner.
   - Three actions: "Accept all" / "Reject non-essential" / "Customize" (granular per-category toggles).
   - Categories: Essential (always on, no choice), Analytics, Marketing. NO opaque grouping.
   - Choice persisted in `cookieconsent_v1` cookie + localStorage (12-month expiry).
   - Until choice is made: analytics + marketing scripts blocked at load-time (not just data-suppression).
   - "Privacy preferences" footer link re-opens the customize modal.
   - No third-party banner library — write our own (~120 LOC) to avoid bloat + CSP issues.

2. **Privacy Policy page** at `skycot.com/privacy`.
   - Sections: what we collect, why, who we share with, retention, user rights, contact, last-updated date.
   - **Real content**, not template fluff. Operator-reviewed before merge.
   - Markdown source at `marketing/content/legal/privacy-policy.md` (operator can edit).

3. **Terms of Service page** at `skycot.com/terms`.
   - Sections: who we are, service description, account terms, acceptable use, payment, IP, warranty disclaimer, liability cap, dispute resolution, contact, last-updated date.
   - Same source-of-truth pattern (markdown).
   - Reference Data Processing Addendum (DPA) availability for enterprise.

4. **Data Processing Addendum template** at `skycot.com/dpa`.
   - Standard GDPR Article 28 controller-processor template.
   - Provided as downloadable PDF + inline page version.
   - Boilerplate per ICO/CNIL/EDPB common patterns.

LOC: ~450.

## Files you may touch

**Marketing:**
- `marketing/components/CookieBanner.tsx` (NEW) — banner + customize modal + persistence. ≤180 LOC.
- `marketing/components/CookieBanner.test.tsx` (NEW). ≤80 LOC.
- `marketing/lib/cookie-consent.ts` (NEW) — get/set/listen API. ≤60 LOC.
- `marketing/lib/analytics-gate.ts` (NEW) — wraps analytics loaders, blocks until consent. ≤40 LOC.
- `marketing/app/layout.tsx` — mount banner. ≤10 modified LOC.
- `marketing/app/privacy/page.tsx` (NEW) — privacy policy page. ≤50 LOC.
- `marketing/app/terms/page.tsx` (NEW) — ToS page. ≤50 LOC.
- `marketing/app/dpa/page.tsx` (NEW) — DPA page. ≤50 LOC.
- `marketing/content/legal/privacy-policy.md` (NEW) — privacy policy content. ≤200 LOC.
- `marketing/content/legal/terms-of-service.md` (NEW) — ToS content. ≤250 LOC.
- `marketing/content/legal/dpa-template.md` (NEW) — DPA template. ≤180 LOC.
- `marketing/components/legal/MarkdownRenderer.tsx` (NEW) — reuse + extend from changelog renderer if it exists, otherwise create. ≤80 LOC.
- `marketing/components/Footer.tsx` — add Privacy / Terms / DPA links. ≤5 modified LOC.

**SPA:**
- `web/lib/cookie-consent-check.ts` (NEW) — checks consent before posting analytics events from SPA. ≤30 LOC.

## Files you must NOT touch

- Backend — these are presentation-side regulatory surfaces only.
- `web/app/...` SPA routes other than the analytics-gate hook — SPA is post-signup; consent gates marketing/analytics scripts which are marketing-side.
- Auth or session cookies — those are functional/essential, not subject to consent.

## Acceptance criteria

- [ ] First-visit to `skycot.com` shows banner.
- [ ] "Reject non-essential" sets cookie + closes banner + does NOT load any analytics/marketing scripts.
- [ ] "Customize" opens modal with per-category toggles + save action.
- [ ] Choice persists for 12 months across sessions.
- [ ] "Privacy preferences" footer link re-opens customize modal.
- [ ] `/privacy`, `/terms`, `/dpa` routes render markdown content cleanly.
- [ ] Operator can edit markdown content without code changes.
- [ ] No third-party cookie-banner JS library in `package.json`.
- [ ] Lighthouse accessibility ≥ 95 for banner + modal + legal pages.
- [ ] Banner works with keyboard navigation (Tab, Enter, Escape).
- [ ] No layout shift when banner appears (reserve space or fade-in).
- [ ] `pnpm typecheck && pnpm test && pnpm lint` clean.
- [ ] BUILD_STATUS.md row updated.
- [ ] Operator reviews + approves privacy + ToS markdown content in PR before merge.

## Branch + PR

- Branch: `claude-cookie-banner-and-legal-pages`
- Title: `marketing: GDPR/CCPA cookie banner + privacy + ToS + DPA (claude-cookie-banner-and-legal-pages)`
- Worker verification: `pnpm typecheck && pnpm test && pnpm lint` per SWARM_PROTOCOL.md. **Operator review explicitly required for legal-page content before merge.**

## Hidden constraints

- **Not legal advice.** The markdown templates are reasonable boilerplate but the operator MUST review against jurisdiction. Add a top-of-file comment: "Reviewed by counsel: [pending / YYYY-MM-DD]".
- **Block analytics at load, not at fire.** Easy bug: load `analytics.js` and only suppress events — that still leaks IP / fingerprint via the GET request itself. Don't load the script until consent given.
- **No dark patterns.** Equal visual prominence on Accept vs Reject. NOT a giant green Accept + tiny grey Reject. Regulators are actively enforcing this.
- **Cookie-name discipline.** `cookieconsent_v1` — bumping the version invalidates old choice (e.g. when adding a new category).
