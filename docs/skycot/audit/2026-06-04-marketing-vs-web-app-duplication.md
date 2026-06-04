# Finding — two frontend apps with overlapping routes + LIVE pricing drift

**Date:** 2026-06-04 · **Author:** Windows lane B infra lane (claude) · **Status:** ⚠️ OPERATOR DECISION REQUIRED
**Raised to:** conductor (via `docs/skycot/agents/SHARED.md`) → operator.
**Trigger:** operator asked "what is Vercel used for?" → investigating surfaced a duplicated-frontend architecture + a public-facing pricing discrepancy.

## TL;DR (the decision)

There are **two separate Next.js apps** with **five overlapping route families**, and they have
already **drifted on the Pro price**: the public marketing site says **$49/mo**, the product +
canonical config + Stripe checkout say **$29/mo**. A prospect on `skycot.com` sees $49; the
product charges $29. **The operator needs to decide (a) the immediate price fix and (b) the
structural fix** for the duplication so this can't recur.

## The architecture (verified)

| App | Package | Domain | Role |
|---|---|---|---|
| `marketing/` | `skycot-marketing` | `skycot.com` (apex) + `www` | Public marketing site (MDX content: about, blog, features, pricing, legal). `marketing/README.md`: *"Separate from the product SPA in `web/`… should not be merged."* |
| `web/` | (the SPA) | `app.skycot.com` | The full product: landing → signup → onboarding → the authenticated dashboard (`(app)/` = team, workers, audit, skills, workflows, billing, …). Has `web/vercel.json`. |

Both deploy to **Vercel** (apex DNS → Vercel A-records; `app.` → Vercel CNAME). The gateway
(`api.skycot.com`) is the AWS Fargate deploy. Funnel: `marketing/` links to `/signup` +
`app.skycot.com` (top-of-funnel → product). So Vercel hosts the **entire** customer UI, not
just a landing page.

## The overlap (verified)

Routes that exist in **both** `marketing/app/` and `web/app/`:

- `pricing`, `about`, `contact`, `customers`, `changelog`
- plus `marketing/app/legal` overlaps `web/app/{privacy,terms,security,sla}`.

So `skycot.com/pricing` (marketing) and `app.skycot.com/pricing` (web) are **two different
pages on two domains**, each a separate source of truth.

## The drift — LIVE pricing discrepancy (the smoking gun)

| Source | Pro price | Evidence |
|---|---|---|
| `marketing/` (PUBLIC, skycot.com) | **$49/mo** | `marketing/app/pricing/page.tsx:54` → `price="$49"` |
| `web/` (PRODUCT, app.skycot.com) | **$29/mo** | `web/app/pricing/page.tsx:76` → `price: "$29"` (+ FAQ :146 "$29/month") |
| `docs/skycot/pricing/v1-tiers.toml` (CANONICAL) | **$29** | `[pro] price_monthly_usd = 29` — drives the Stripe checkout |

**`web/` + canonical agree ($29); the public marketing site is wrong ($49).**

### Why it drifted (maintenance asymmetry)

- `marketing/`: 10 commits total, **last touched 2026-05-24** (the $49 is ~11 days stale —
  almost certainly the original price before it was lowered to $29).
- `web/` marketing-type routes: 30 commits, **last touched 2026-06-04** (today — `web/pricing`
  just got the Stripe checkout wiring in WS4 #2677).

`web/` is actively maintained; `marketing/` is lagging and nobody updated its hardcoded price
when the canonical price changed to $29.

## Why it matters

- **Commercial / credibility.** A prospect on `skycot.com` sees **$49**, clicks through to
  `app.skycot.com`, and the product + checkout say **$29**. Best case they're confused; worst
  case they bounce at $49 or feel baited. This is exactly the "can't be called out or
  embarrassed" class of issue for a paying-ready launch.
- **Structural drift risk.** Five duplicated route families means every shared fact (price,
  legal terms, company copy) has two homes that can silently disagree. Pricing is just the one
  that's already broken; about/contact/customers/legal are the same hazard.

## NOT verified (honest caveats)

- I did **not** live-probe `skycot.com` to confirm `marketing/` is currently deployed there —
  inferred from DNS (apex → Vercel) + `marketing/README.md`. If `marketing/` is **not** live
  yet, the $49 hasn't shipped publicly and this is a pre-launch fix, not a live bug.
- I confirmed drift on **pricing only**. The other four overlapping routes
  (about/contact/customers/changelog) are duplicated and drift-prone but I did not diff their
  content.
- `web/app/(app)/marketing/` is a *product feature* (the in-app GTM/marketing agent), **not**
  the marketing website — unrelated to this finding.

## The decision the operator needs to make

**(1) Immediate — the price.** Confirm Pro = **$29** (canonical) and fix
`marketing/app/pricing/page.tsx:54` `$49 → $29`. One-line change; can ship now independent of
the structural decision. (Or, if $49 is actually the intended new price, fix `web/` +
`v1-tiers.toml` + Stripe instead — but that contradicts the just-shipped checkout, so $29 is
almost certainly correct.)

**(2) Structural — how to stop duplication recurring.** Options:

- **A. Keep two apps, single-source the shared facts.** Drive pricing from `v1-tiers.toml`
  (or a shared package) in *both* apps so price can't diverge; same for legal. Lowest churn;
  preserves the MDX content-site separation. Still leaves duplicated about/contact pages to
  reconcile by hand.
- **B. One app at both domains.** Serve `web/` at the apex too and delete `marketing/`.
  Simplest single-source-of-truth, but loses the deliberate MDX-content-site split and makes
  the apex a heavier app. Contradicts `marketing/README.md`'s explicit "do not merge."
- **C. Strict ownership split.** Marketing routes live **only** in `marketing/`; `web/` keeps
  **only** product routes (delete `web/app/{pricing,about,contact,customers,changelog}`, or
  redirect them to `skycot.com`). Each route has exactly one home. Cleanest long-term; needs
  the funnel re-pointed (e.g., the product's "see pricing" links → `skycot.com/pricing`).

**Infra-lane recommendation (not a product call):** ship the **$29 hotfix now** regardless,
then pick **A or C**. A is least disruptive; C is the cleanest "one home per route." B fights
an explicit design decision. The operator owns the call.

## Suggested follow-up briefs (for the conductor to author after the operator decides)

- `fix-marketing-pricing-29-not-49` — the immediate hotfix (frontend, 1 line). Independent.
- `reconcile-marketing-vs-web-duplicate-routes` — implements the chosen A/B/C (frontend).
- (If A) `pricing-single-source-from-v1-tiers-toml` — both apps read one price source.
