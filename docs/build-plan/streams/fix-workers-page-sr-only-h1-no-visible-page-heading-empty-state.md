# fix-workers-page-sr-only-h1-no-visible-page-heading-empty-state

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workers-page-sr-only-h1-no-visible-page-heading-empty-state`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F17 [P3].

🟢 **P3 — The `/workers` page wraps `<HireStarterPrompts />` inside an `<h1 className="sr-only">Workers</h1>` — visually invisible. The `TeamEmptyState` heading "Hire your first AI teammate" is `<h2>`, not the page's `<h1>`. Screen readers are fine, but the empty state loses its editorial-typography moment — the operator who just landed here from onboarding sees no "Workers" page title, just the empty card.**

## Root cause

`web/app/(app)/workers/page.tsx:39` renders the page heading as `<h1 className="sr-only">Workers</h1>` — visible only to screen readers. The actual visible heading is `TeamEmptyState`'s `<h2>Hire your first AI teammate</h2>`. Result: the page has correct accessibility (sr-only h1 exists) but visually there's no page header, just the empty card.

For a user who just landed here from onboarding, the missing "Workers" header undercuts the wayfinding — they don't see the page transition reinforce its identity. Per memory `project_workers_page_design_locked` (C2-holographic-showcase): "viral/wow register, gradient banners, animated shine, huge weight-900 level numbers." The empty state should still carry that register, not just disappear.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F17 [P3] — `/workers` page has its `<h1>` hidden (`sr-only`) — no visible page heading on the empty state

## Fix recipe

1. **Promote the heading hierarchy.** In `web/app/(app)/workers/page.tsx:39`, two options:
   - **(a)** Remove the `sr-only` class — make the "Workers" h1 visible at the top of the page, then the `TeamEmptyState` h2 stays as section heading. Cleanest.
   - **(b)** Keep the page h1 sr-only AND promote `TeamEmptyState` heading to h1, making it the page's primary heading both visually and semantically. Then drop the sr-only redundancy.
2. **Recommend (a)** — preserves the page-vs-section-heading distinction; matches the "Your team" / "Workers" page header design pattern used on most other in-app pages.
3. **Apply C2-holographic register.** When the page has zero workers, render a small page header ("Workers" or "Your team — empty") above the empty card, using the same Audiowide font + gradient eyebrow as the populated state. Don't make the empty state visually inferior to the populated one.
4. **Verify no double-h1.** After the change, the rendered DOM must have exactly one h1. Lint via `grep -c '<h1' web/app/(app)/workers/page.tsx` returning 1.
5. **Vitest pin** — assert the rendered page has a visible "Workers" h1 in both empty and populated states.

## Acceptance criteria

- [ ] `/workers` (both empty and populated) renders a visible "Workers" or "Your team" h1 at the top.
- [ ] The DOM has exactly one h1 element.
- [ ] Empty state renders the page header above the empty card (not just the card alone).
- [ ] Vitest pin asserts the page-header presence in both states.
- [ ] Screen reader announces "Workers, heading level 1" (not "list of buttons").

## May-touch

- `web/app/(app)/workers/page.tsx` (header + h1 promotion)
- `web/components/skycot/TeamEmptyState.tsx` (possibly demote h2 to h3 if hierarchy needs shifting)
- `web/components/skycot/HireStarterPrompts.tsx` (verify hierarchy)
- `web/app/(app)/workers/__tests__/page.test.tsx` (new or extend)

## Out of scope

- The `/team` page (separate route — its duplicate-h1 issue covered by `fix-team-page-duplicate-h1-and-settings-shell-h1-hierarchy`)
- Empty-state gendered language (covered by `fix-team-empty-state-gendered-language-her-to-them`)
- The `/workers` → `/team` redirect logic (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`)

## Effort

XS — ~10-20 LOC. ~20-30 min. P3 because no demo scene shows the empty `/workers` state today.
