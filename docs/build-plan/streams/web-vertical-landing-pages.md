# web-vertical-landing-pages — /for/sales, /for/recruiting, /for/support, /for/finance, /for/marketing, /for/ops

**Owner**: claude. **Sizing**: ~150 LOC per page × 6 = ~900 LOC total.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-vertical-landing-pages`.
**Priority**: P1 — every vertical-specific search ("AI agent for sales", "AI receptionist", etc.) needs a tailored landing page to convert. The single /landing page is too generic. Six vertical landings = six SEO entry points + six tailored conversion stories.

## Scope

Six new routes under `/for/<vertical>`, each a vertical-specific marketing landing page. They share the structure + brand register of `/landing` but with vertical-tailored copy, hero, workflow examples, and customer quotes.

### Pages to ship

1. `/for/sales` — "Your AI sales team that never sleeps."
2. `/for/recruiting` — "Hire faster. Screen better. Never lose a candidate."
3. `/for/support` — "Customer support that gets it right the first time."
4. `/for/finance` — "Close the books. Catch the variance. Sleep at night."
5. `/for/marketing` — "Always-on campaigns, never-off-brand."
6. `/for/ops` — "Run the back office on autopilot."

### Shared structure (per page)

Reuse the locked `/landing` layout (`web/app/landing/page.tsx`) — extract a `<VerticalLandingShell>` component that takes vertical-specific props if it makes the diff cleaner; otherwise duplicate the structure per file and accept the LOC cost.

Each page has:

1. **Hero** — Audiowide headline (vertical-tailored), 1-sentence subhead, primary CTA (Start free) + secondary (See it work).
2. **Trust strip** — same as /landing (SOC 2 / GDPR / N companies this month).
3. **"Built for <vertical> teams" section** — 3 cards: the 3 most-painful workflows for that vertical, with a one-sentence outcome statement each.
   - Sales: "Qualify inbound leads", "Draft outreach sequences", "Book demos automatically"
   - Recruiting: "Screen incoming applications", "Schedule first-round interviews", "Reference-check candidates"
   - Support: "Triage tickets by urgency", "Draft replies in your tone", "Escalate when stuck"
   - Finance: "Extract invoices", "Reconcile vendor payments", "Flag expense anomalies"
   - Marketing: "Brand-voice QA on outgoing copy", "Lead enrichment from form fills", "Campaign performance digest"
   - Ops: "Run weekly status reports", "Sync data across CRM/ERP", "Audit your skill catalog"
4. **Workflow showcase** — embed a screenshot or HTML mockup of one signature workflow from that vertical's template gallery. Caption: "Here's how it runs." Link goes to `/workflows/templates?vertical=<v>`.
5. **Customer quote** — placeholder for now (use a real one when available; lorem-style stub is acceptable as long as it's marked `data-pending-quote` for easy find-replace).
6. **CTA strip** — gradient bar with "Hire your first <vertical> teammate" + Start free button.
7. **Footer** — same as /landing.

### Branding

- Each vertical gets a subtle accent color overlay on the hero (light tint) drawn from the existing brand palette — purple for sales, cyan for recruiting, amber for support, green for finance, pink for marketing, gray-blue for ops. Don't break the locked brand identity; this is an accent overlay, not a re-skin.
- Audiowide on the hero word only (signature SKYCOT brand-loud register).
- Inter for body, weight 200 for hero subhead.

### Nav

- Update `/landing`'s top nav to include a "For" dropdown with links to all six.
- Each vertical page has the same nav so cross-navigation works.

### SEO

- Each page sets a unique `<title>` and `<meta description>` via Next.js metadata export.
- Each page has a unique OG image path (e.g. `og-sales.png` — placeholder file is fine; image generation is v1.x).

## Acceptance criteria

- [ ] `/for/sales`, `/for/recruiting`, `/for/support`, `/for/finance`, `/for/marketing`, `/for/ops` all render.
- [ ] Each page has Audiowide hero with vertical-tailored copy.
- [ ] Each page has the 3-workflow-cards section with vertical-specific examples.
- [ ] Each page has a workflow showcase image/mockup section linking to the template gallery filtered by vertical.
- [ ] /landing top nav has a "For" dropdown linking to all six.
- [ ] Each page sets a unique `<title>` + `<meta description>`.
- [ ] Mobile-responsive on each page (test at 375px width).
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Real customer quotes / logos (placeholder stubs marked `data-pending-quote` are acceptable).
- A/B testing infrastructure for variant copy — v1.x.
- Per-vertical demo videos — v1.x.
- Sales/recruiting/etc. detail pages beyond the single landing per vertical — those belong to `/customers/<slug>` or `/templates/<v>`.
