# fix-workflows-page-hero-copy-doesnt-deliver-on-how-they-think

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workflows-page-hero-copy-doesnt-deliver-on-how-they-think`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F11)

🟡 **P2 — `/workflows` h1 reads "How your teammates / actually think." in brand-loud-editorial register, but the page body is a flat directory of 8 workflow names. The body doesn't deliver on the "how they think" promise of the hero — it's more like "Workflows directory". Demo Scene 5 visits /workflows; the hero copy reads as theatre to a careful viewer.**

## Root cause

`web/app/(app)/workflows/page.tsx` renders a brand-loud Audiowide H1 ("How your teammates / actually think.") above a flat 8-card workflow list. The H1 promises a peek into the reasoning behind the workflows; the body shows names, versions, and substrate chips — useful but not reasoning. Memory `project_workflow_goldens_page_design_locked` locked a richer layout for the goldens page; this brief brings the headline page closer to that promise.

Two paths to fix:

- **Path A**: Tighten the heading to match what the page actually delivers ("Your workflows").
- **Path B**: Add an editorial intro band above the list that DELIVERS on the "how they think" promise — a 1-2 paragraph plain-English explanation of "here are the decision patterns your teammates use" with a 'go deeper' link to the goldens page or workflow detail.

Per memory `project_brand_consistency_pass`, brand-loud surfaces (this is one) should carry the D3 recipe — Audiowide hero is correct, but the editorial follow-through is missing.

## Findings this brief closes

- whole-site F11 [P2] — /workflows hero brand-loud but body is flat list; doesn't deliver on "how they think"

## Fix recipe

Recommend **Path B** for the demo (it preserves the brand-loud moment AND delivers content); Path A as the fallback.

**Path B — add an editorial intro band.**

1. **Insert an editorial band** between the H1 and the workflow card grid at `web/app/(app)/workflows/page.tsx`:
   - 2-paragraph plain-English intro that describes the workflow concept in human terms:
     - Paragraph 1: "Every workflow your teammates run is a sequence of decisions — fetch this, summarise that, escalate if X, approve via Y."
     - Paragraph 2: "We show you the whole decision chain. Click any workflow to see step-by-step what runs, who authored it, and which decisions need your approval."
   - A small "Open the goldens library →" link to `/workflows/goldens` or wherever the goldens-capture page lives.
   - Use Inter weight 300/400 for the body (not Audiowide); the Audiowide is reserved for the H1.
2. **Layout**: editorial band sits in a max-width container (~720px) centred, with the workflow card grid expanding back to full-width below.
3. **Mobile**: stack as expected; the editorial copy should be readable at 375px.
4. **Storybook + vitest**:
   - Story for the new editorial band variant.
   - vitest asserts the editorial band renders and the H1 stays as-is.
5. **Coordinate with the locked design memory** — this is an additive content beat that respects the existing register; flag the change in the PR body so it's clear no design-lock is being broken.

## Acceptance criteria

- [ ] `/workflows` renders an editorial intro band between the H1 and the workflow list.
- [ ] The band delivers 2 paragraphs of plain-English copy that explains the "how they think" promise.
- [ ] The H1 brand-loud register (Audiowide + gradient) is preserved.
- [ ] Mobile (375px) renders the band stacked above the list without horizontal overflow.
- [ ] vitest + storybook pass.
- [ ] PR body cites whole-site F11 and notes the additive nature.

## May-touch

- `web/app/(app)/workflows/page.tsx` (~40 LOC additive)
- `web/app/(app)/workflows/page.test.tsx` (NEW or extend, ~30 LOC)
- `web/app/(app)/workflows/page.stories.tsx` (extend with editorial-band story)

## Out of scope

- The 8 workflow cards themselves (covered by sibling workflow-card briefs)
- The /workflows hero brand-consistency more broadly (covered by `project_brand_consistency_pass`)
- The mock-bypass pill on the eyebrow (covered by `fix-workflows-page-mock-pill-eyebrow-on-camera`)

## Effort

XS/S — ~40 LOC copy + ~30 LOC tests. ~1-2 hours.
