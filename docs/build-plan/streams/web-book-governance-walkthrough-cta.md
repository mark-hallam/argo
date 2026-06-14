# web-book-governance-walkthrough-cta

**Authored against:** origin/main `20fbc6b8c` on 2026-06-15
**Validate before claim:** `bash scripts/check-brief-freshness.sh web-book-governance-walkthrough-cta`
**Priority:** P2 — GTM-enabling (turns the new trust positioning into enterprise
pipeline). Not a ship blocker, but the highest-leverage GTM build per the
[2026-06-15 launch research](../../skycot/research/2026-06-15-launch-readiness-competitive-pricing-gtm-compliance.md).

## Problem

The landing now leads with the research-validated positioning ("provably honest
agents" — `5c578141b`), and the recommended GTM motion is **enterprise/sales-led**
(trust/governance wedge to the C-suite). But the site's only conversion path is the
**waitlist** (a PLG/self-serve email capture) — there is **no sales-led "talk to us /
book a walkthrough" path**, which is where trust-differentiated enterprise deals are
actually won (the security/procurement conversation starts with a call). The hero
already reframes the waitlist toward "a governance walkthrough" but has no real
booking destination behind it.

## Design (operator-decided 2026-06-15: hybrid form → scheduler, graceful-degrade)

Add a **"Book a governance walkthrough"** primary CTA in the landing hero (sales-led),
alongside the existing waitlist (now the secondary/self-serve feeder). The flow:

1. **Short qualifying form** (2–3 fields: work email, company, role + one free-text
   "what would you hand to an AI teammate?"). POST to the **EXISTING
   `/api/v1/enterprise-inquiry` endpoint** — reuse it, do NOT build a parallel route.
   (Grep first: `web/components/marketing/` likely already has an enterprise-inquiry /
   waitlist form pattern + the anon-POST middleware exemption from #2886 — extend it.)
2. **On success → show a scheduler embed** so the qualified prospect self-books a slot.
   The scheduler is **tool-agnostic via env var** `NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL`
   (Calendly or Cal.com inline embed — operator drops in the URL).
3. **NO-THEATER graceful degrade:** if `NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL` is
   unset, the form submission alone IS the inquiry — show an honest "Thanks — we'll be
   in touch to schedule your walkthrough" state. **Never** render a "book" affordance
   that promises a calendar that isn't wired (the lesson from the dead-play-button work
   #2892).

## Why hybrid (not scheduler-only or form-only)
- Scheduler-only → tire-kicker bookings waste founder time + no lead data in your system.
- Form-only → adds a "we'll follow up to schedule" delay that loses high-intent buyers.
- Hybrid → lead data (in `/api/v1/enterprise-inquiry`) **and** a booked meeting in one
  flow, with context for the call. Standard "Book a demo" pattern.

## Verifier (checkable, NO THEATER)
- vitest (mirror `web/app/landing/page.test.tsx`): the "Book a governance walkthrough"
  CTA renders; submitting the form POSTs to `/api/v1/enterprise-inquiry`; on success
  with `NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL` set → the scheduler embed renders; unset
  → the honest "we'll be in touch" state renders and **no** scheduler/booking affordance
  is shown. No dead handler, no pre-faked success.
- `pnpm build` / tsc clean; landing tests stay green.

## May-touch
- `web/app/landing/page.tsx` (hero CTA), a new `web/components/marketing/WalkthroughCta.tsx`
  (or extend the existing enterprise-inquiry form component), `web/lib/api/*` (reuse the
  enterprise-inquiry fetcher), the env-var plumbing. Brand/design LOCKED — copy + the
  embed only, consume existing tokens.

## Out of scope
- Building our own scheduling engine (use an embed). Changing the brand/design system.
- CRM integration beyond what `/api/v1/enterprise-inquiry` already does (follow-up brief
  if wanted).

**LOC budget:** ~150–300 (web only).
