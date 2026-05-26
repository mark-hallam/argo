# fix-onboarding-pick-vertical-demo-mode-chip-clutter

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-pick-vertical-demo-mode-chip-clutter`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/onboarding-hire-an-ai-teammate.md` F9 [P2].

🟡 **P2 — Each of the 7 vertical cards on `/onboarding/pick-vertical` ends with a yellow `DEMO MODE · INSTALL DEFERRED` chip beneath the skill-count line. Honest per `feedback_no_theater_in_demo` (each chip tells the truth about install state), but visually heavy — adds ~30% vertical height to every card. On a 7-card grid the page becomes scroll-heavy and the chip repetition reads as noise. Operator's eye is drawn to the repeating yellow chip rather than the vertical content; the affordance reads as "this is broken" rather than "this is demo mode".**

## Root cause

`web/app/(setup)/onboarding/pick-vertical/page.tsx` renders 7 vertical cards. Each card's footer includes:

```tsx
<div className="card-footer">
  <span>{skillCount} SKILLS</span>
  {isDemoMode() && (
    <Chip variant="warning">DEMO MODE · INSTALL DEFERRED</Chip>
  )}
</div>
```

The chip is correct individually — it accurately reports the install-deferred state for each card. But the SAME chip on 7 cards is:

1. **Redundant** — the operator only needs to know "we're in demo mode" once, not seven times.
2. **Visually loud** — yellow + warning variant + repeating pattern = strong attentional pull, drawing the eye away from the vertical names and descriptions which are the load-bearing content of the step.
3. **Scroll cost** — adds ~30% vertical height per card. On a 7-card grid (likely 2 rows × 4 columns or 3+3+1), that's a measurable extra scroll for the operator + camera.

The cleanest fix: move the "demo mode · install deferred" message ONCE to a banner at the top of the page (above the grid), and remove the chip from each card. Operator reads the banner once, then scans clean cards.

This is dressing-side polish, but the camera lands on this page during Scene 3, so the visual heaviness reads as "the product is half-built" rather than "we're in demo mode for the recording".

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P2] — Step-4 vertical-pick "Skill count · DEMO MODE · INSTALL DEFERRED" chip is honest but visually busy

## Fix recipe

1. **Add a single info-banner at the top of `web/app/(setup)/onboarding/pick-vertical/page.tsx`**, above the vertical grid:
   ```tsx
   {isDemoMode() && (
     <InfoBanner
       variant="info"
       icon={InfoIcon}
       data-testid="pick-vertical-demo-banner"
     >
       Demo mode — install deferred. Picking a vertical loads its workflows + skills as samples; nothing is provisioned to the gateway until you connect a real provider in Settings.
     </InfoBanner>
   )}
   ```
2. **Remove the per-card chip** at the card-footer level. Each card's footer now reads just `{skillCount} SKILLS` (no chip).
3. **Verify the install-deferred state is honest** — the banner copy must accurately describe what demo-mode pick-vertical actually does. Cross-check against `fix-onboarding-pick-vertical-demo-mode-honest-install-relabel`'s recipe to ensure copy alignment.
4. **Visual regression**: snapshot the grid before/after via Playwright; assert vertical scroll height drops measurably (>15% reduction).
5. **Vitest**: render the page in demo mode → assert the banner is in DOM, the per-card chip is NOT.

## Acceptance criteria

- [ ] `/onboarding/pick-vertical` in demo mode shows ONE info banner above the grid.
- [ ] No per-card "DEMO MODE · INSTALL DEFERRED" chip on any of the 7 vertical cards.
- [ ] Banner copy is honest about what install-deferred means in demo mode.
- [ ] Visual scroll height of the grid reduced by ≥15%.
- [ ] No regression on the pick-vertical step's mechanics.

## May-touch

- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (banner + chip removal)
- `web/components/ui/InfoBanner.tsx` (reuse existing or NEW — matches design lock)
- Vitest in `web/app/(setup)/onboarding/pick-vertical/__tests__/`
- Playwright visual regression in `web/playwright/onboarding/`

## Out of scope

- Pick-vertical install-deferred actual behavior (covered by `fix-onboarding-pick-vertical-demo-mode-honest-install-relabel`).
- Skill counts per vertical (covered by `fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog`).
- Vertical card design D3-consistency pass (separate brief).
- Wizard polish cluster (covered by `fix-onboarding-wizard-polish-cluster-wrap-scroll-eyebrow`).

## Effort

~40-60 LOC: banner add + per-card chip removal + vitest. ~30 min. P2 because dressing-side polish but on Scene 3's camera-visible step.
