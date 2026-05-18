# mission-marketing-hero-card-layout — fix feature-card text-stacking at large viewports (P2)

**Owner**: claude. **Sizing hint**: tiny CSS fix.
**Depends on**: none.
**Authored against**: origin/main `91f9d032` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-marketing-hero-card-layout`.
**Source**: 2026-05-18 browser UX audit — at 1920px viewport the three feature cards under the hero stack their headlines vertically: "Workers / with / memory.", "Approve / in the / tools you already / use.", "Auditable / by / default." The cards have a narrow max-width that forces the headline to wrap word-by-word. Looks unintentional.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — first impression of the product is the marketing landing; broken-looking text is the wrong first impression.

## Scope

P2 fast-mode. Adjust the feature-card grid so headlines fit naturally (likely: widen the max-width per card, or shrink the headline font, or both).

LOC: ~30.

## Files you may touch

- `marketing/components/ui/FeatureSection.tsx` (or wherever the 3-up card layout lives — grep `Workers with memory` to find). ≤20 modified LOC.
- The component's CSS / Tailwind classes for max-width + grid-cols. ≤10 modified LOC.

## Files you must NOT touch

- The card content (copy + icons) — pure layout fix.
- Other cards / sections of the landing.

## Acceptance criteria

- [ ] At 1920x1080 viewport, "Workers with memory.", "Approve in the tools you already use.", "Auditable by default." render on at most 2 lines each (one line preferred).
- [ ] At 1440px, 1280px, 1024px viewports, layout still looks correct.
- [ ] Mobile (375px, 414px, 768px) layout unchanged.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-marketing-hero-card-layout`
- Title: `fix(marketing): feature card headline fits cleanly at 1920px (mission-marketing-hero-card-layout)`
- Worker verification: manual viewport check at 5+ widths.

## Hidden constraints

- **Don't redesign the cards** — pure stacking fix. If the design needs a bigger rethink, that's a separate brief.
- **The fix may also benefit the stats row** (which had similar stacking observable just below the cards on first screenshot). If it's the same CSS pattern, fix once.
