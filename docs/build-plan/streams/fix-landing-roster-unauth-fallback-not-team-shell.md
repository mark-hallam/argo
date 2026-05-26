# fix-landing-roster-unauth-fallback-not-team-shell

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-landing-roster-unauth-fallback-not-team-shell`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/whole-site-mvp-claims-coverage.md` F6 [P1].

🟠 **P1 — `/landing` top nav links "Roster" → `/team`. `/team` is an in-app shell route that renders the sidebar nav + an empty-state "Hire your first teammate" CTA. A logged-out visitor landing there sees a half-broken authenticated surface — the marketing-to-product handoff is brittle.**

## Root cause

`/landing` MarketingNav includes `<a href="/team">Roster</a>`. `/team` resolves under the `(app)` route group at `web/app/(app)/team/page.tsx`, which assumes the visitor has a session. With no session, the page renders the in-app shell (sidebar nav, navbar, demo banner) plus an empty-state CTA "Hire your first teammate" — fine if you're logged in and have no workers, but a confusing entry for a marketing visitor.

Either:
- (a) Change the marketing nav link to `/cohorts` (per the locked design memory, `/cohorts` is the canonical roster surface). But `/cohorts` has the same problem — it's also under `(app)`.
- (b) Build a public marketing-mirror at `/teammates` or `/roster` that renders a brand-restrained public-friendly subset (no sidebar, no in-app chrome).
- (c) Add a logged-out fallback at `/team` that detects no-session and renders a marketing-style view inline.

Recommended: (b) — a separate marketing-mirror keeps responsibility clean and doesn't pollute the in-app route with logged-out branching.

## Findings this brief closes

- whole-site-mvp-claims-coverage F6 [P1] — `/landing` nav links to `/team` but `/team` is an in-app shell, not a marketing roster page

## Fix recipe

1. **Build `/teammates`** (new marketing route) at `web/app/teammates/page.tsx`:
   - Renders inside the marketing layout (no sidebar, no demo banner).
   - Hero matches `/landing` register; copy: "Meet the team you'll hire" / "Five teammates the SKYCOT platform ships with on day one. Real personas with real responsibilities — see who's already on the team."
   - Render the 5 canonical personas (Mariah, Peta, Priya, Jamal, Nori) using `SAMPLE_PERSONAS` from `web/lib/sample-personas.ts`.
   - Use a brand-restrained variant of the C2 card (no MOCK-OFFLINE pill, no live-status, no level numbers — purely descriptive).
   - CTA at the bottom: "Build your own team — Try a demo workspace" → `/auth/signup`.

2. **Update the MarketingNav link** to point at `/teammates` instead of `/team`. Files to check:
   - `web/components/skycot/marketing/MarketingNav.tsx` (or wherever the marketing nav is defined)
   - Footer navs that reference `/team`.

3. **Optional follow-up** — also gate `/team` itself to redirect unauthenticated visitors to `/teammates`:
   ```tsx
   // web/app/(app)/team/page.tsx
   const session = await getSession();
   if (!session) redirect("/teammates");
   ```
   Only do this if the operator agrees that the in-app `/team` should never serve unauthenticated traffic.

4. **Add a Playwright spec** `web/playwright/marketing/teammates-public-roster.spec.ts`:
   - Visit `/teammates` with no session cookie.
   - Assert HTTP 200.
   - Assert the page contains the 5 persona names.
   - Assert NO sidebar / demo-banner is visible.
   - Assert the CTA links to `/auth/signup`.

## Acceptance criteria

- [ ] `/teammates` returns 200 to logged-out visitors and renders the 5 personas inside the marketing layout.
- [ ] MarketingNav "Roster" link points at `/teammates`, not `/team`.
- [ ] No sidebar / demo banner appears on `/teammates`.
- [ ] CTA links to `/auth/signup`.
- [ ] New Playwright spec passes.
- [ ] In-app `/team` for logged-in visitors is unchanged.

## May-touch

- `web/app/teammates/page.tsx` (NEW)
- `web/app/teammates/layout.tsx` (NEW or extends marketing layout)
- `web/components/skycot/marketing/MarketingNav.tsx` (link target)
- Optionally `web/app/(app)/team/page.tsx` (unauth redirect)
- `web/components/skycot/PublicPersonaCard.tsx` (NEW — brand-restrained variant of the C2 card)
- `web/playwright/marketing/teammates-public-roster.spec.ts` (NEW)

## Out of scope

- Customer logo fictions (covered by `fix-landing-page-fictitious-customer-logos-and-testimonials`).
- Marketing nav fragment anchors (covered by `fix-marketing-nav-fragment-anchors-and-dead-cta`).
- The "Add `/team` to marketing nav" brief — `fix-marketing-nav-add-team-link-and-customer-page-robots` predates this finding and asks for the link to be added; this brief instead redirects the request to `/teammates`. Coordinate with that brief at claim-time.

## Effort

~200-400 LOC for the new page + spec. ~4-8 hours. P1 — POC demo doesn't visit `/teammates`, but it's the first surface an investor's second tab will hit.
