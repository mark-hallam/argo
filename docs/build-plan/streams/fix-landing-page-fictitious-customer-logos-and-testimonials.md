# fix-landing-page-fictitious-customer-logos-and-testimonials

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-landing-page-fictitious-customer-logos-and-testimonials`.
**Source**: e2e-ux-audit Agent 1 findings F1 + F2 + F3 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md`).

🟡 **P1 — landing page has 12 fictitious customer logos + 3 fictitious testimonials + a fake "2,847 teams this month" pill. Many logos don't even back to `/customers` stories.**

## Root cause

Three coupled landing-page issues, dressing-side per the function-vs-dressing rule but with internal inconsistencies that break the contract:

1. **`web/app/landing/page.tsx:63-99`** — "Already shipping with SKYCOT" → 12 made-up customer names (Northwind Commerce, Acmeco Retail, Polaris Marketing, Zenith Support, Harbour Legal, Meridian Logistics, Atlas Manufacturing, Lumen Education, etc.). **7 of the 12 logo cards reference companies that don't appear on `/customers`** (which has 5 case studies, all 5 with story pages). The 7 orphans break the dressing-side contract: a viewer hovering them sees a customer that doesn't exist anywhere else in the product.

2. **Testimonials by fictitious people**: "Daniela Reyes (Head of CX, Northwind Commerce)", "Marcus Lin (CCO, Beacon Health)", "Aisha Patel (Ops Director, Polaris Marketing)". Same dressing-side accept-if-internal-consistency-holds rule — none of these back to a real `/customers` story.

3. **`web/app/landing/page.tsx:177`** — "2,847 teams this month" pill in the trust strip. Reads as a real number, alongside legitimate compliance badges (SOC 2-READY AUDIT TRAIL · ISO 27001 · GDPR/HIPAA). The four security badges are honest; the team-count pill is a lie sitting next to them.

4. **`web/app/landing/page.tsx:182-196`** — hero art bubbles reference Mariah/Priya/Jamal/Nori as illustrative personas. Operator memory `project_pm_cohorts_peta_patrick` introduces Peta + Patrick. If the demo MP4 cuts from `/landing` → `/team`, Maya appears in isolation while the marketing page hyped Mariah et al. Visual mismatch (dressing-acceptable but inconsistent).

## Findings this brief closes

- Agent 1 F1 [P2] — Landing hero stat "2,847 teams this month" reads as a real number
- Agent 1 F2 [P1] — 12 fictitious customer logos + 3 fictitious testimonials on `/landing`
- Agent 1 F3 [P2] — `/landing` hero illustration uses Mariah/Priya/Jamal/Nori not Maya (persona mismatch with `/team`)

## Fix recipe

Three changes, one PR:

1. **Trim LOGOS to the 5 with backing case studies** at `web/app/landing/page.tsx:63-99`. The 7 orphans get deleted entirely OR marked "case study coming" (with a non-link state, no hover affordance).

2. **Trim testimonials to ≤ the number of real case studies**. Either drop all 3 (cleanest — no testimonial section until the operator has real customer quotes), or restrict to 1 per real customer story.

3. **Replace the "2,847 teams this month" pill** with one of:
   - The operator's real cohort count (memory says current swarm hasn't accumulated meaningful customers yet)
   - A different trust pill (e.g. "EARLY ACCESS · APPLY")
   - Remove the pill — the four security badges already carry the trust register.

4. **Align hero art-bubble personas with canonical roster**. Show Mariah/Peta/Priya/Jamal/Nori (the canon 5 per `fix-demo-seed-cli-for-real-roster`) — not Maya alone (she's the marketing-intern singular fixture for now). When the demo MP4 cuts from `/landing` → `/team`, all 5 should appear together (post-seed).

## Acceptance criteria

- [ ] `/landing` LOGOS section has only customers that have backing pages on `/customers` (≤5 entries).
- [ ] No testimonials that don't back to a `/customers` story.
- [ ] No "X,XXX teams this month" pill (or it cites a real number).
- [ ] Hero art-bubble personas match the canon roster: Mariah/Peta/Priya/Jamal/Nori.
- [ ] No regression on Lighthouse / Core Web Vitals for the landing page.
- [ ] Screenshot diff in PR showing the trimmed surface.

## May-touch

- `web/app/landing/page.tsx:63-99` (logo strip)
- `web/app/landing/page.tsx:177` (trust pill)
- `web/app/landing/page.tsx:182-196` (hero art bubbles)
- Any testimonial JSX between those line ranges

## Out of scope

- Building real customer case study pages (separate brief if needed)
- Brand-style updates (locked per memory `project_skycot_brand_identity`)
- The styled-jsx vs server-component constraint (per memory `feedback_styled_jsx_requires_use_client`)

## Effort

~50-150 LOC. ~1-2 hours.
