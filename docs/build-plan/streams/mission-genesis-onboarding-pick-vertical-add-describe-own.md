# mission-genesis-onboarding-pick-vertical-add-describe-own

**Owner**: claude.
**Authored against**: origin/main `5b20282f7` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-onboarding-pick-vertical-add-describe-own`.

## Why (verified gap, 2026-06-09)
DEC-034 WS3: **Genesis IS the hire path** — a stranger describes the teammate they want and watches it get built. The Genesis root shipped (`/genesis` SPA page at `web/app/(app)/genesis/page.tsx` calling `POST /api/v1/genesis` SSE; FEATURE_MATRIX row "Genesis SSE + classify + draft-worker" ✅ Live, browser-verified). But the onboarding **step-04 vertical picker** never routes there.

VERIFIED on current `origin/main`:
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` renders ONLY canned-template cards (`cards.map(...)` from `useVerticals()`, lines 292-327) plus the back-CTA. There are **0 occurrences of `genesis`** in that file (`rg -li genesis` on the dir returns nothing). The hero copy is literally "Pick a starting template." (lines 252-253).
- So a stranger walking onboarding sees a static template picker — the "describe the teammate, watch it built" moment that distinguishes SKYCOT is unreachable from the hire flow.
- `/genesis` exists and is the real describe-and-watch surface (`web/app/(app)/genesis/page.tsx`), but nothing in onboarding links to it.

This is the missing CTA that turns the picker into the front door of Genesis.

## Grep-before-write
- `rg -n 'genesis' 'web/app/(setup)/onboarding/pick-vertical/page.tsx'` — confirm still 0 hits before editing (re-verify; do not duplicate if a prior worker already added it).
- `rg -l 'GenesisDescribeCta|DescribeYourOwnCta|GenesisCta' web/` — confirm the proposed component name is free (verified free 2026-06-09).
- `rg -n 'ob-role-card|ob-roles|ob-cta-row' web/app/globals.css web/app/**/*.css` — reuse the wizard's existing `ob-*` token classes; do NOT invent a parallel card style.
- `rg -n "router.push\\('/genesis'|href=\"/genesis\"" web/` — see how other callers route to `/genesis` and match the convention.
- `rg -n 'useSearchParams|return=|searchParams' 'web/app/(app)/genesis/page.tsx'` — check whether `/genesis` consumes a return param TODAY before sending one (do not invent an unconsumed param).
- `rg 'pub (struct|enum|fn)' crates/` — N/A: this brief is web-only (TSX). No new Rust public surface is declared.

## Fix (additive; consumer = the onboarding wizard)
1. **Add a Genesis CTA above the template grid** in `web/app/(setup)/onboarding/pick-vertical/page.tsx`. A single distinct card/button — "Describe your own teammate" with an "AI will build it" sub-label + sparkle icon — rendered ABOVE the `<ul className="ob-roles">` grid (line 292). Extract it into a NEW small presentational component `web/components/skycot/onboarding/pick-vertical/GenesisDescribeCta.tsx` so the page (already 353 LOC) stays under the 500-LOC ceiling. Style with the existing `ob-*` tokens + one gradient-border accent (the only brand-loud accent in the otherwise-restrained wizard, per memory `project_onboarding_design_locked` F1).
2. **Route into Genesis.** Clicking it `router.push("/genesis")`. Carry a return hint ONLY if `/genesis` already reads one (see grep above); if it does not, route plain `/genesis` and leave a one-line TODO rather than inventing an unconsumed param.
3. **Keep the templates.** The canned cards stay as the fast-lane; Genesis is the headline path. The demo-mode (`isDemoMode()`) short-circuit is untouched — the CTA renders in both modes as the only new element.
4. **Mobile (375px):** the CTA stacks cleanly above the grid (reuse the grid's responsive container; no new media query).

## Acceptance (truth-bounded)
1. `web/app/(setup)/onboarding/pick-vertical/page.tsx` now renders a Genesis CTA element with `data-testid="onboarding-genesis-describe-cta"` that `router.push("/genesis")` on click; the template cards still render (`onboarding-vertical-grid` unchanged).
2. New/updated **vitest** (web's Jest/Vitest runner): render the page (or the `<GenesisDescribeCta>` unit), assert the CTA is present with the right testid and that a click navigates to `/genesis` (mock `useRouter`). The existing grid test still passes.
3. The repo's web test runner is green for the touched files; web lint clean on touched files.
4. Both touched files stay < 500 LOC (extract the CTA so the page does not cross the ceiling).

## On ship (same commit)
Update the FEATURE_MATRIX.md Genesis / onboarding row to note the onboarding picker now offers a "describe your own teammate" entry into Genesis. 🚨 DEC-043: do NOT regenerate or commit `feature-matrix.html` (gitignored) — `.md` row only.

## Effort
S — web-only, ~80-130 LOC (one new presentational component + a few lines in the page + a vitest). ~1-2h.
