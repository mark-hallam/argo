# web-customer-case-study-pages — /customers index + /customers/<slug> detail pages

**Owner**: claude. **Sizing**: ~250-400 LOC + content templates.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-customer-case-study-pages`.
**Priority**: P1 — SKYCOT has zero social proof on the public site today. Every enterprise prospect asks "who else uses this?" and the answer should be a credible /customers page. This brief ships the structure now so case studies can drop in as they materialize.

## Scope

Two new public-facing routes:

### 1. `/customers` — index page

- Brand-loud register (matches /landing / /skills hero treatment).
- Audiowide hero: "Companies running on SKYCOT."
- Subhead: 1 sentence stat about category leadership ("From sales teams of 12 to ops teams of 1,200, SKYCOT is the AI-teammates platform for serious businesses.").
- Grid of 6 customer cards. Each card:
  - Customer logo (placeholder SVG or PNG — store under `web/public/customers/<slug>.svg`)
  - Company name + 1-line industry tag ("E-commerce · 200 employees")
  - Pull-quote (1-2 sentences from the customer)
  - Headline KPI ("Resolved 73% of inbound tickets without a human", "Closed the books 4 days faster", etc.)
  - "Read story" CTA → `/customers/<slug>`
- Card layout: 3-column grid on desktop, 2-column on tablet, 1-column on mobile.
- Below the grid: "Get featured here" CTA strip → mailto link or contact form.
- Initial seed: 6 placeholder customer entries with `data-pending-real-quote` markers so future replacement is trivial.

### 2. `/customers/<slug>` — per-customer detail page

Brand-loud register. Structure per detail page:

1. **Hero strip** — customer logo + name (large) + industry/size tag + headline KPI ("Resolved 73% of inbound tickets without a human").
2. **Quote pull** — large editorial pull-quote from the customer rep with attribution (name + title).
3. **The setup** — 3-4 paragraph editorial story: what they were doing before SKYCOT, what changed.
4. **Workflow showcase** — an embedded workflow card (reuse the visual workflow viewer component) showing the signature workflow this customer runs. Caption + "See the template" link → `/workflows/templates/<workflow-id>`.
5. **Metrics block** — 3 large KPI tiles (volume processed, time saved, cost saved or whichever 3 numbers the customer is happy to share).
6. **Second quote** — shorter, from a different stakeholder (engineering / ops / CEO).
7. **CTA strip** — "Want this for your team?" + Start free.

### 3. Content templates

Provide 6 placeholder customer entries as a typed const array in `web/lib/content/customers.ts`:

```ts
export const SEED_CUSTOMERS: CustomerStory[] = [
  {
    slug: "acme-co",
    name: "Acme Co",
    industry: "E-commerce",
    size: "200 employees",
    logoUrl: "/customers/acme-co.svg",
    headlineKpi: "73% of inbound tickets resolved without a human",
    pullQuote: "We hired 12 AI teammates in a week. They never miss a follow-up.",
    quoteAuthor: "Jane Doe, COO at Acme Co",
    // ... full story fields
    dataPending: true, // marker for real-replacement
  },
  // ... 5 more
];
```

Each placeholder has a unique-feeling persona so the page reads believably until real ones land.

### 4. Routing + types

- `web/app/(public)/customers/page.tsx` — index page.
- `web/app/(public)/customers/[slug]/page.tsx` — detail page (server component, reads slug, renders from `SEED_CUSTOMERS`).
- `web/lib/content/customers.ts` — typed seed data + `CustomerStory` interface.
- 404 if `slug` doesn't match any seed entry.

### 5. SEO

- Each detail page sets a unique `<title>` and `<meta description>` via Next.js metadata.
- Each has a unique OG image path (placeholder file fine).
- Top-nav on /landing gets a "Customers" link added.

## Acceptance criteria

- [ ] `/customers` renders with 6 placeholder customer cards (logo + name + KPI + quote + CTA).
- [ ] Each card links to a working `/customers/<slug>` detail page.
- [ ] Each detail page has the 7-section structure (hero / quote / setup / workflow showcase / metrics / second quote / CTA).
- [ ] Workflow showcase embeds a working workflow visualization (reuse existing viewer component).
- [ ] /landing top nav has a "Customers" link.
- [ ] All placeholder content tagged `dataPending: true` for easy bulk replacement.
- [ ] Mobile-responsive at 375px.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Real customer logos / quotes / metrics (operator + sales own this; placeholders unblock the structure).
- Customer-self-serve case-study submission UX — v1.x.
- Filtering or search inside /customers — v1.x.
- Carousel / video embeds inside detail pages — v1.x.
