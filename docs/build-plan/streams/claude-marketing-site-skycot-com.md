# claude-marketing-site-skycot-com — Marketing site at skycot.com (Wave 3 P0)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED + `claude-design-system-component-extension` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-marketing-site-skycot-com`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **The customer's first impression.** If `skycot.com` looks like vapor, the entire product is judged as vapor.
- **All 6 mission claims need to be communicated** — the marketing site is the persuasion surface for them.

## Scope

P0. Separate Next.js marketing site at `marketing/` directory (distinct from `web/` which is the SPA at `app.skycot.com`).

Pages:
1. **`/` (home)** — Hero ("Hire a teammate, not a tool"). 3 feature highlights. KeyFacts row. Sign-up CTA. Footer.
2. **`/features`** — All 6 mission claims as feature sections with diagrams.
3. **`/pricing`** — 3 tiers (Free / Pro / Enterprise) using PricingCard component.
4. **`/customers`** — Testimonials (3 placeholder design-partner slots; real ones drop in post-launch).
5. **`/changelog`** — v1.0 release notes + roadmap. Updated weekly.
6. **`/about`** — Mission statement. Team (operator + advisors). Investor page (optional, gated).
7. **`/contact`** — Email + Discord + form for enterprise inquiries.
8. **`/legal/privacy`** — Privacy policy (template + customized).
9. **`/legal/terms`** — Terms of service (template + customized).

Each page:
- Inter Display headlines, Inter body.
- Jonny-Ive discipline: restraint, materials honesty, beautiful typography.
- Mobile-first responsive (test 360px).
- Dark/light mode auto-toggle (respect system preference).
- Lighthouse ≥ 95 all metrics.
- A11y: axe-core clean.
- SEO: meta tags, OpenGraph cards, JSON-LD structured data, sitemap.xml, robots.txt.

LOC: ~2000.

## Files you may touch

**New marketing site (separate Next.js app):**
- `marketing/package.json` (NEW) — Next.js 15 + same design system. ≤40 LOC.
- `marketing/next.config.ts` (NEW). ≤30 LOC.
- `marketing/tailwind.config.ts` (NEW) — shares brand tokens with web/. ≤30 LOC.
- `marketing/postcss.config.mjs` (NEW). ≤10 LOC.
- `marketing/tsconfig.json` (NEW). ≤30 LOC.
- `marketing/app/layout.tsx` (NEW) — shared layout: Nav + Footer + theme. ≤120 LOC.
- `marketing/app/globals.css` (NEW) — imports brand tokens. ≤40 LOC.
- `marketing/app/page.tsx` (NEW) — home. ≤180 LOC.
- `marketing/app/features/page.tsx` (NEW). ≤220 LOC.
- `marketing/app/pricing/page.tsx` (NEW). ≤150 LOC.
- `marketing/app/customers/page.tsx` (NEW). ≤120 LOC.
- `marketing/app/changelog/page.tsx` (NEW). ≤120 LOC.
- `marketing/app/about/page.tsx` (NEW). ≤140 LOC.
- `marketing/app/contact/page.tsx` (NEW). ≤120 LOC.
- `marketing/app/legal/privacy/page.tsx` (NEW). ≤300 LOC (privacy policy text).
- `marketing/app/legal/terms/page.tsx` (NEW). ≤300 LOC (terms of service text).
- `marketing/app/sitemap.ts` (NEW). ≤30 LOC.
- `marketing/app/robots.ts` (NEW). ≤15 LOC.
- `marketing/components/` (NEW dir) — marketing-specific composites built from web/components/ui primitives. ≤200 LOC combined.
- `marketing/public/og.png` (NEW) — OpenGraph image. (Generated; placeholder ok for first cut.)

**Workspace integration:**
- Root `package.json` — narrowly: add marketing as workspace if pnpm/yarn workspaces used. ≤10 modified lines (check existing structure).
- `pnpm-workspace.yaml` if needed. ≤5 modified lines.

## Files you must NOT touch

- `web/` (SPA) — separate code path. Do NOT import marketing components into SPA or vice versa.
- Backend Rust — out of scope.
- Brand tokens in `web/app/globals.css` — copy + share, don't modify.

## Acceptance criteria

- [ ] 9 pages render correctly on `localhost:3001` (or whatever port).
- [ ] Mobile-responsive: tested at 360px viewport.
- [ ] Dark/light mode toggle works + persists.
- [ ] Lighthouse ≥ 95 on home page (test on `next build && next start`).
- [ ] A11y: axe-core reports zero violations.
- [ ] SEO: meta + OG + sitemap + robots all present.
- [ ] Privacy + Terms pages have real content (lawyer review can come later but text in place).
- [ ] Build succeeds: `cd marketing && pnpm build`.
- [ ] `pnpm lint && pnpm type-check` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-marketing-site-skycot-com`
- PR title: `feat(marketing): skycot.com marketing site (claude-marketing-site-skycot-com)`

## Notes for the implementer

- **Use existing design system.** Import `Nav`, `Footer`, `LandingHero`, `FeatureSection`, `PricingCard`, `KeyFact`, `TestimonialCard`, `CodeBlock`, `Diagram` from `web/components/ui/`.
- **Hero copy:** Headline: "Hire a teammate, not a tool." Sub: "SKYCOT generates AI workers from plain-English job descriptions. They run in Slack, learn from your feedback, and ask for approval like a human would." CTA: "Hire your first worker" → `app.skycot.com/signup`.
- **Privacy + Terms:** start with templates from termly.io or termsfeed; customize for SKYCOT specifics. Real lawyer review post-launch.
- **Voice & tone:** read `docs/skycot/brand/IDENTITY.md` (from `claude-brand-identity-system`). NO exclamation points. NO emoji clutter. Direct, intelligent.
- **Imagery:** use the 5 architecture SVG diagrams from `docs/build-plan/build-pdf/diagrams/` on features page. Real screenshots from app.skycot.com on home + features.
- **Pricing copy:** straight from `docs/skycot/pricing/v1-tiers.toml`. Don't duplicate or rewrite the source of truth.
- After this brief lands, operator deploys to Vercel + maps `skycot.com` apex.
