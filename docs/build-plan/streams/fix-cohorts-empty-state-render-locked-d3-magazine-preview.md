# fix-cohorts-empty-state-render-locked-d3-magazine-preview

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cohorts-empty-state-render-locked-d3-magazine-preview`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) findings F6 + F13 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1/P2 — `/cohorts` empty state shows a generic empty SaaS card instead of any preview of the locked D3 magazine layout. The empty-state CTA "Create cohort" links to `/cohorts/new` which 404s.**

## Root cause

**F6** (P1): Memory `project_cohorts_page_design_locked` describes "6 dept tabs + asymmetric lead/stacked/compact (1 lead + 2 stacked + 4 compact)". When `cohorts.length === 0` the page (`web/app/(app)/cohorts/page.tsx:79-90`) shows the generic `<EmptyState illustration={CohortsIllustration} headline="No cohorts" cta={{label:"Create cohort"}}>` instead of any preview of the locked layout.

For a fresh-tenant demo, `/cohorts` looks identical to any generic empty SaaS page. Weakens the "Platform builds workers" claim if an investor probes the sidebar.

**F13** (P2): The empty-state CTA `<EmptyState cta={{label:"Create cohort", href:"/cohorts/new"}}>` (`web/app/(app)/cohorts/page.tsx:87`) points to `/cohorts/new` which is not a defined route in `web/app/(app)/cohorts/` (only `page.tsx`, `layout.tsx`, `error.tsx`, `page.test.tsx`). Clicking the CTA will 404 into the brand-themed not-found page.

Two related findings — empty state is wrong shape AND the empty-state CTA is broken.

## Findings this brief closes

- whole-site-mvp-claims-coverage F6 [P1] — `/cohorts` empty state hides the locked D3 magazine-tile design
- whole-site-mvp-claims-coverage F13 [P2] — `/cohorts/new` CTA route doesn't exist

## Fix recipe

Two options for F6; pick (a) for the cleanest fix:

**(a) Seed the demo tenant with at least one cohort**:
1. Have the demo-seed CLI (`crates/argo-cli/src/bin/demo-seed.rs`) mint 6 cohorts (one per dept tab: GTM / Marketing / Sales / Support / Operations / Project) with the canonical workers assigned.
2. With the seed, the empty state never fires for the demo tenant; the locked D3 layout always renders.
3. Tie into `fix-demo-seed-cli-for-real-roster` brief — same code path.

**(b) Render a "preview-mode" magazine grid in the empty state**:
1. Touch site: `web/app/(app)/cohorts/page.tsx:79-90`.
2. Replace the `<EmptyState>` block with the D3 magazine layout populated with sample-only cohort tiles, with a subtle "Demo · click to create your own" treatment.
3. Add a "Create your first cohort" CTA above the preview.
4. Risk: looks like real data when it isn't (theatre, against operator-locked `feedback_no_theater_in_demo`). Don't pick (b) unless the sample tiles are clearly marked.

Recommend (a). The demo lane wants real data; option (a) is consistent with the operator's no-theater rule.

**For F13 (cohorts/new route)**:
1. Build `web/app/(app)/cohorts/new/page.tsx` with the cohort-creation wizard (name + dept + initial workers).
2. OR change the empty-state CTA to open a modal-based create flow on the current page.
3. Pick (1) for parity with how other surfaces handle creation (workers, workflows).

## Acceptance criteria

- [ ] `/cohorts` on the demo tenant renders the D3 magazine layout (at least 1 cohort visible).
- [ ] The CTA "Create cohort" lands on a working `/cohorts/new` page OR opens a modal.
- [ ] No 404 from the empty-state CTA.
- [ ] Pin: `web/tests/cohorts-page-renders-d3.spec.ts` — assert at least 1 cohort tile renders; assert "Create cohort" CTA destination doesn't 404.

## May-touch

- `web/app/(app)/cohorts/page.tsx`
- `web/app/(app)/cohorts/new/page.tsx` (NEW)
- `crates/argo-cli/src/bin/demo-seed.rs` (if option a)
- `web/components/skycot/cohorts/CohortCreateForm.tsx` (NEW)
- `web/tests/cohorts-page-renders-d3.spec.ts` (NEW)

## Out of scope

- Cohort detail real-data wire (separate brief `mission-cohort-detail-real-data-wire`)
- Cohort skill bulk-assign (separate brief `mission-cohort-detail-bulk-assign-skill`)
- Cohorts loading skeleton (separate brief `web-cohorts-magazine-tile-loading-skeleton`)

## Effort

~150-300 LOC (depending on whether `/cohorts/new` is a full page or modal). ~3-5 hours.
