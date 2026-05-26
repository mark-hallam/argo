# fix-onboarding-welcome-headline-singular-teammate-vs-workforce

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-headline-singular-teammate-vs-workforce`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F5 + F6.

🟡 **P1 — Two copy + interaction defects on `/onboarding/welcome` break the linear hire-flow narrative the camera follows. (1) The headline `Build your AI workforce.` switches from the landing page's singular "Hire a teammate" framing to fleet/tool framing — the same vocabulary drift the marketing-cta-teammate-vs-worker memory locked against. (2) Pillar 2 "Onboard them like a human" is a clickable link that deep-routes to `/genesis` mid-wizard with no breadcrumb back, ejecting the user from the 5-step flow at the second step.**

## Root cause

`web/app/(setup)/onboarding/welcome/page.tsx:21-24` defines `heroSegments` with the literal string `"AI workforce"` as the gradient phrase. The landing page (per locked brand memory + marketing-cta vocab brief) sells *one named teammate* — the welcome step contradicts that with plural "workforce" framing, snapping back to fleet/tool language. Operator-locked memory `project_onboarding_design_locked` describes the F1 editorial design but doesn't lock the headline copy, so this drift slipped.

`web/app/(setup)/onboarding/welcome/page.tsx:43-62` mounts three pillars in `<button role="link" tabIndex={0}>` shells with `onClick` + `onKeyDown` handlers. Pillar 2 specifically (`"Onboard them like a human"`) routes to `/genesis` — a top-level studio surface — but the route does not preserve the onboarding draft, does not breadcrumb back, and does not advance the wizard step. A demo viewer or buyer who clicks the pillar (the affordance reads "interactive") loses the linear narrative and ends up on a sibling product surface mid-recording.

Genesis is the operator-locked **AI-teammate onboarding studio** (memory `feedback_genesis_is_ai_teammate_onboarding_studio`) — i.e. the headline investor moment. Surfacing it as a click-through inside another linear flow splits the narrative and competes with the wizard for attention.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F5 [P1] — Onboarding/welcome vocab clashes with landing vocab ("AI workforce" vs "Hire a teammate")
- onboarding-hire-an-ai-teammate F6 [P1] — Welcome pillar "Onboard them like a human" deep-links to `/genesis` mid-wizard

## Fix recipe

1. **Rewrite the welcome headline to singular framing.** `web/app/(setup)/onboarding/welcome/page.tsx:21-24` heroSegments — change the segment array so the headline reads `Hire your first teammate.` with the gradient on `first teammate`. Preserve the existing Audiowide + radial-blob + gradient treatment from the locked F1 design — only the words change.
2. **Drop the click handler on pillar 2** (and ideally all three pillars — they are not the wizard's CTAs and competing-for-click is the problem). Remove `role="link"`, `tabIndex`, `cursor: pointer`, `onClick`, `onKeyDown` from the pillar containers at lines 43-62. Pillars become passive marketing affordances, exactly like the locked D3 landing pillars.
3. **Optionally add an explicit "Use Genesis instead" link** below the `Get Started` CTA, with copy along the lines of "Have a stack of role-docs to upload? **Try Genesis studio →**". This preserves the discoverability operator wanted for Genesis, but pulls it out of the pillar-as-link affordance so it doesn't compete with the wizard's primary path. Wire as a regular `<Link>` to `/genesis?return=/onboarding/welcome` so Genesis can offer a "back to onboarding" breadcrumb when the return param is present (Genesis-side change out of scope; this brief only adds the link).

## Acceptance criteria

- [ ] `/onboarding/welcome` h1 reads "Hire your first teammate." (or operator-approved equivalent that keeps singular framing). Gradient on "first teammate".
- [ ] The three pillars on `/onboarding/welcome` are no longer keyboard-focusable as links / no longer respond to click. `aria-role="link"` is removed.
- [ ] Playwright pin: visiting `/onboarding/welcome` and pressing Tab from the page hero skips the pillars and lands on the primary CTA.
- [ ] Vitest pin: the pillar 2 element has no `onClick` prop.
- [ ] (Optional, low risk) A "Try Genesis studio →" link appears below the primary CTA with `href="/genesis?return=/onboarding/welcome"`.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx` (heroSegments + pillar role/click removal + optional Genesis link)
- `web/app/(setup)/onboarding/welcome/__tests__/page.test.tsx` (new pin if missing)

## Out of scope

- Brand-voice rewrite of the pillar bodies (covered by `fix-onboarding-welcome-pillars-bridge-not-repitch.md` + `fix-onboarding-welcome-hero-flat-brand-voice-drift.md`)
- Genesis-side `?return=` breadcrumb wiring (separate brief if operator wants it: `fix-genesis-honor-return-query-param`)
- Step-strip eyebrow polish (covered by `fix-onboarding-welcome-eyebrow-step-one-pattern.md`)

## Effort

XS — ~30-50 LOC. ~30-60 minutes. P1 because it's the second-screen drift after `/landing` and the camera will read it.
