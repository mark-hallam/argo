# web-hire-done-celebration-brandloud — wrap /hire/done in BrandLoudHero + confetti

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-hire-done-celebration-brandloud`.
**Priority**: P2 — wave-5 viral-moments: hire-done is small PartyPopper icon + plain heading. Operator's first-hire payoff moment should be the viral-screenshot peak.
**Sizing**: ~200-350 LOC (BrandLoudHero variant + confetti library + tests).

## Why this brief exists

From the wave-5 viral-moments audit:

> Hire-done celebration — small PartyPopper icon + `text-2xl`
> heading is the entire payoff for hiring your first AI teammate.
> Fix: wrap in `<BrandLoudHero>` + confetti.

The user's first emotional moment after signing up: hiring their
first teammate. Currently lands as a generic SaaS confirmation
screen. Per operator brand promise ("UX must be epic, must go
viral") this is exactly the moment to land hard.

## Goals

1. Wrap /hire/done/[workerId] in `<BrandLoudHero>` with the new teammate's name as the gradient-accent.
2. Confetti animation on mount (purple→pink particles).
3. Polished welcome card: avatar (or initials) + name + role + "Say hi in Slack" CTA + "View profile" CTA.
4. Reduced-motion guard on confetti.
5. Audiowide hero copy: "Meet Mariah." or similar — uses the just-hired worker's name.

## Steps

1. Read existing `/hire/done/[workerId]/page.tsx` (currently bare).
2. Use lightweight confetti — either install `canvas-confetti` (~10KB) or build a CSS-only burst.
3. Wrap in BrandLoudHero with a custom Audiowide title ("Meet ${name}.")
4. Add the welcome card below.
5. Wire CTAs: "Open in Slack" deep-link (mock URL OK for v1), "View profile" → /workers/[workerId]/overview.
6. Reduced-motion guard.

## may-touch

- `web/app/(hire)/hire/done/[workerId]/page.tsx`
- New: `web/components/skycot/hire/HireDoneCelebration.tsx`
- Possibly install: `canvas-confetti` (npm add)

## must-NOT-touch

- Other /hire steps (only the done page)
- BrandLoudHero (reuse as-is)

## Acceptance criteria

- [ ] /hire/done/wkr_XXX renders a BrandLoudHero with the worker's name in Audiowide gradient.
- [ ] Confetti animation fires on mount (skipped with prefers-reduced-motion).
- [ ] "Open in Slack" + "View profile" CTAs wire correctly.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Viral-moments re-audit: /hire/done wow-factor 9+.

## References

- Audit: `docs/skycot/audit/viral-moments/2026-05-20-wave-5-viral-moments.md`
- Memory: `user_design_exploration_optometrist_method.md` ("UX must be epic, must go viral")
