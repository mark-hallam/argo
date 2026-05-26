# fix-onboarding-wizard-footer-trust-strip-fictional-companies-count

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-footer-trust-strip-fictional-companies-count`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F9.

🟡 **P2 — Hardcoded "2,847 COMPANIES THIS MONTH" trust-strip in the bottom footer of every onboarding wizard step. The number is specific, fictional, and inconsistent with the landing page's honest "founding 100" eyebrow. Per operator-locked `feedback_no_theater_in_demo`: dressing is OK if the camera doesn't drill in, but a number this specific in a v1 product creates a screenshot-diff vector for journalists/skeptics down the line.**

## Root cause

`web/components/auth/WizardEditorialShell.tsx:418-420` hardcodes a trust-strip element with the literal text `2,847 COMPANIES THIS MONTH`. The number is not sourced from any backend metric, not bound to a feature flag, not honest about v1 reality. The landing page already lands the honest framing ("Founding 100 · Early access") per operator-locked brand memory — the wizard footer contradicts it.

The footer is in the camera-readable region on every step (small text but legible at 1080p). Even if the camera doesn't drill into it, it's the kind of fixture that becomes a public-credibility issue when (a) the press takes a screenshot, (b) the count visibly doesn't move week to week, (c) competitors mock it.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P2] — Wizard-footer trust strip claims "2,847 COMPANIES THIS MONTH"

## Fix recipe

1. **Replace the hardcoded fixture with the honest landing-page eyebrow.** In `web/components/auth/WizardEditorialShell.tsx:418-420`, swap the trust-strip content from `2,847 COMPANIES THIS MONTH` to `<strong>Founding 100</strong> · EARLY ACCESS COHORT`. Matches the locked landing eyebrow exactly.
2. **Alternative if operator wants to preserve a metric:** route the count through a real backend metric. Out-of-scope for this brief — file a follow-on `fix-trust-strip-real-count-from-gateway` if pursued.
3. **Sweep other camera-readable hardcoded counts.** Per the audit, this is the only one found in the wizard chrome. Verify no sibling fixture exists elsewhere in `WizardEditorialShell.tsx` or in the `(setup)/layout.tsx` shell.

## Acceptance criteria

- [ ] Wizard footer trust-strip text reads "Founding 100 · EARLY ACCESS COHORT" (or operator-approved equivalent).
- [ ] No hardcoded fictional company/customer count appears anywhere in `WizardEditorialShell.tsx` or `(setup)/layout.tsx`.
- [ ] Vitest snapshot pin: the trust-strip element does NOT contain the literal `2,847`.
- [ ] Landing page eyebrow and wizard footer trust-strip read the same brand framing.

## May-touch

- `web/components/auth/WizardEditorialShell.tsx` (footer trust-strip)
- `web/app/(setup)/layout.tsx` (verify no sibling fixture)
- `web/components/auth/__tests__/WizardEditorialShell.test.tsx` (snapshot pin)

## Out of scope

- The landing-page roster fallback (covered by `fix-landing-roster-unauth-fallback-not-team-shell.md`)
- Customer-logo placeholders (covered by `fix-landing-page-fictitious-customer-logos-and-testimonials.md`)
- Real-metric backend wiring (file `fix-trust-strip-real-count-from-gateway` if pursued)

## Effort

XS — ~10-20 LOC. ~20 minutes. P2 because dressing-side but credibility-relevant for the public demo.
