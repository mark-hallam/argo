# fix-workflows-build-entry-page-brand-consistency-d3-recipe

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workflows-build-entry-page-brand-consistency-d3-recipe`
**Source:** UX e2e audit 2026-05-28-0252 (scene-4-and-5 F19)

🟡 **P2 — `/workflows/build` "Workflow builder" header uses a bare Tailwind-classed header (`text-3xl font-semibold tracking-normal`) with no Audiowide, no warm bg, no gradient — it feels like a placeholder from before the brand-consistency pass. Memory `project_brand_consistency_pass` says all 13 in-product surfaces should carry the D3 recipe; this one slipped.**

## Root cause

`web/app/(app)/workflows/build/page.tsx:23-34` renders a bare Tailwind hierarchy:

```tsx
<h1 className="text-3xl font-semibold tracking-normal">Workflow builder</h1>
```

with no Audiowide font, no purple→pink gradient on the heading, no warm radial bg, no `#1e1b4b` ink. The locked design recipe per memory `project_brand_consistency_pass` should apply.

The 50-template gallery (`/workflows/templates`) DOES carry the recipe (Audiowide eyebrow + gradient hero + warm bg); this entry page does not. Inconsistent register within the same product surface.

## Findings this brief closes

- scene-4-and-5 F19 [P2] — `/workflows/build` brand-inconsistency; bare Tailwind header instead of D3 recipe

## Fix recipe

1. **Port the D3 recipe** from `/workflows/templates` to `/workflows/build`:
   - **Eyebrow**: small Audiowide font (~13px) reading `BUILD A WORKFLOW` or `WORKFLOW STUDIO` in purple gradient.
   - **H1**: Audiowide font, ~48-56px, with the purple→pink gradient on a keyword (e.g. "Build a workflow.").
   - **Body bg**: warm radial blob bg (`var(--background-radial)` per the consistency pass).
   - **Ink**: `#1e1b4b` for body text.
   - **Subhead**: Inter weight 300, ~18px, plain-English explanation of what the builder does.
2. **Apply the same recipe to the 5-vertical chooser cards** below the hero — match the cohort-D3 magazine tile aesthetic the rest of the product uses.
3. **Sibling brief coordination**: `fix-workflows-build-vertical-picker-add-marketplace` (IN_REVIEW #2537) and `fix-workflows-build-vertical-picker-add-marketing-and-compliance` (IN_REVIEW #2470) widen the vertical list. This brief is the visual register fix; coordinate via brief comment so they don't conflict.
4. **Storybook + vitest**:
   - Storybook for the new branded layout.
   - vitest pins the eyebrow text + H1 font class + gradient class.

## Acceptance criteria

- [ ] `/workflows/build` renders Audiowide eyebrow + brand-gradient H1.
- [ ] Page bg uses the warm radial recipe consistent with `/workflows/templates`.
- [ ] 5-vertical chooser cards match the cohort-D3 magazine register.
- [ ] Mobile (375px) renders correctly without horizontal overflow.
- [ ] vitest + storybook pass.
- [ ] PR body cites scene-4-and-5 F19 and links to memory `project_brand_consistency_pass`.

## May-touch

- `web/app/(app)/workflows/build/page.tsx` (~80 LOC layout rewrite)
- `web/app/(app)/workflows/build/page.test.tsx` (NEW or extend, ~40 LOC)
- `web/app/(app)/workflows/build/page.stories.tsx` (extend or NEW)
- Optionally `web/components/skycot/workflow/BrandedHero.tsx` (NEW if extracting the shared hero shell)

## Out of scope

- Vertical picker list itself (covered by sibling vertical-picker briefs IN_REVIEW)
- Sales-tile redirect (covered by `fix-workflows-build-sales-tile-redirect-to-onboarding`)
- Workflow canvas / builder behaviour (separate scope)
- `/workflows/templates` register (already on the recipe)

## Effort

S — ~80 LOC layout + ~40 LOC tests + storybook. ~2-3 hours.
