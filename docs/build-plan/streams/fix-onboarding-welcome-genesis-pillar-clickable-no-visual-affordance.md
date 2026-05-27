# fix-onboarding-welcome-genesis-pillar-clickable-no-visual-affordance

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-genesis-pillar-clickable-no-visual-affordance`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F18 [P3].

🟢 **P3 — `/onboarding/welcome` makes the middle pillar ("Onboard them like a human") a clickable `<div role="link">` that routes to `/genesis`. The other two pillars are static. There's no visual cursor cue beyond `style={{ cursor: "pointer" }}`, no hover state, and the screen reader gets `role="link"` but a sighted user has no signal this is a hotspot.**

## Root cause

`web/app/(setup)/onboarding/welcome/page.tsx:43-62` renders the middle pillar as:

```tsx
<div role="link" onClick={() => router.push("/genesis")} style={{ cursor: "pointer" }}>
  <h3>Onboard them like a human</h3>
  ...
</div>
```

Problems:
- The other two pillars are static (no link). The middle one is interactive but looks identical.
- No hover state, no underline, no chevron/arrow, no "Try Genesis →" CTA copy.
- `role="link"` is semantically correct but visually the pillar is undiscoverable.
- Per memory `feedback_genesis_is_ai_teammate_onboarding_studio`, Genesis is the headline investor moment. The pillar SHOULD draw attention; today it's accidentally hidden.

The fix is straightforward: make the clickable pillar visually distinct from the static ones — gradient border, hover scale/glow, explicit "Try Genesis →" affordance.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F18 [P3] — Genesis pillar on welcome step is a `role="link"` div clickable via Enter/Space but doesn't visually look clickable

## Fix recipe

1. **Convert the clickable pillar to a proper `<Link>` button.** Use Next.js `<Link href="/genesis">` wrapping a styled card. Drop `role="link"` and `onClick` — use semantic HTML.
2. **Add visual affordance.** Distinguish the clickable pillar from the static two:
   - Add an explicit `Try Genesis →` chip/CTA inside the card (using the locked brand purple→pink gradient)
   - Add a hover state: subtle scale + glow + chevron-translate-on-hover
   - Add a gradient border or accent ring on hover/focus
3. **Test keyboard focus** — pressing Tab should land focus visibly on the clickable pillar; pressing Enter should navigate to `/genesis`.
4. **Mobile**: tap target should be ≥ 44 × 44 px. Hover state should degrade gracefully on touch.
5. **(Optional)** Add a small "investor-moment" badge: "★ Featured" or similar, to draw attention to Genesis as the headline beat.

## Acceptance criteria

- [ ] `/onboarding/welcome` middle pillar uses a proper `<Link>` with semantic HTML.
- [ ] Visual hover state distinguishes the clickable pillar from the static two.
- [ ] Tab-focus lands visibly on the clickable pillar with a focus ring.
- [ ] Enter key navigates to `/genesis`.
- [ ] Tap target ≥ 44 × 44 px on mobile viewport (375 px wide).
- [ ] No regression on the other two static pillars.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx:43-62`
- `web/components/skycot/welcome/Pillar.tsx` (if extracted)
- CSS / Tailwind classes for the hover state

## Out of scope

- Welcome step h1 copy (covered by `fix-onboarding-welcome-headline-singular-teammate-vs-workforce`)
- Genesis page itself (covered by `fix-genesis-page-h1-onboarding-studio-not-hire`)
- The other two static pillars (no fix needed)

## Effort

XS-S — ~20-50 LOC + Tailwind tweaks. ~30-45 min. P3 because the welcome step is a brief in-flow moment; investor demo MP4 may not pause on this pillar long enough for the affordance gap to register, but the Genesis call-out IS strategically important per memory.
