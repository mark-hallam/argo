# rebase-fix-demo-mode-banner-investor-safe-copy

Re-implement the original brief
`streams/fix-demo-mode-banner-investor-safe-copy.md` against current
`origin/main` HEAD.

## Original PR

PR #2392 (closed for re-author) — `fix(web): make demo mode disclosure
investor-safe`, opened 2026-05-26 by codex. Conflicted irretrievably on
4 of 5 touched files because the demo-banner surface was re-architected
multiple times since:

- `web/components/skycot/DemoBanner.tsx` — the banner is now `sticky
  top-0 z-30` with `pointer-events:none` on the container and
  `pointer-events:auto` only on the dismiss button (so it doesn't
  intercept clicks on the workflow-builder palette / react-flow nodes
  underneath). It explicitly skips `/onboarding/*` routes per F14
  ("the wizard's own top bar already signals demo intent"). PR #2392's
  rewrite of the banner copy collides with this entire shape.
- `web/components/skycot/DemoPill.tsx` — PR #2392 wanted to add this
  NEW component. The "demo pill" concept has shipped instead via PR
  #2511 (`web: add onboarding demo-mode pill affordance (F18)`) — the
  pill lives on `/onboarding/welcome` with `data-testid="onboarding-demo-pill"`
  and `NEXT_PUBLIC_SKYCOT_DEMO_MODE` gating, NOT via a `BANNER_STYLE=pill`
  environment variable. The PR's variant + env-var design is no
  longer load-bearing.
- `web/lib/skycot/demo-mode.ts` — the file now hosts `recordingModeReadinessWarning`
  + the "MOCK-OFFLINE / MOCK-BYPASS / LIVE" chip surface (gated behind
  opt-in per PR #2623) + canonical-name `NEXT_PUBLIC_SKYCOT_DEMO_MODE`
  consumers. PR #2392's `NEXT_PUBLIC_SKYCOT_DEMO_BANNER_STYLE=pill`
  variant design is orthogonal and conflicts on the same module-level
  exports.
- `docs/skycot/demos/v1-mission-mvp-runbook.md` — heavily re-edited
  multiple times since the PR opened (Scene 6 pre-flight, runbook
  fallback section, Scene 5 CSRF smoke, demo-live opt-in flag
  section). The PR's documentation insertion needs a fresh anchor
  point.

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-demo-mode-banner-investor-safe-copy

## Bug is still load-bearing — narrow scope

The current banner copy in `web/components/skycot/DemoBanner.tsx`
reads:

> Demo mode — showing sample data. Sign in to see your own workspace.

For an investor watching the v1.0 demo MP4, this reads like a marketing
mock — not the operator-locked positioning per
`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` and
`feedback_no_theater_in_demo`: **the demo IS the product, running real
actions in a sandboxed tenant**. The copy under-sells what's
happening on screen.

The pill-variant + environment-flag + 8s auto-dismiss design from the
original PR is NO LONGER load-bearing — PR #2511 already shipped the
onboarding pill on a different surface (the wizard, not the app
shell), and the operator memory `feedback_no_theater_in_demo` says
demo seeded data is REAL (Sam Harlow / Acmeco / six real workers via
the seed CLI per PR #2433 + #2559). So the rebase scope is narrower:
fix the copy on the existing `DemoBanner` only.

## Acceptance (narrowed from original brief)

- `web/components/skycot/DemoBanner.tsx` copy change ONLY. Replace
  `"Demo mode — showing sample data. Sign in to see your own workspace."`
  with one of these candidates (operator picks at PR review time):
  - **A (conservative)**: `"Hosted POC — every action is real in this
    sandboxed tenant. Sign in to talk to your own workspace."`
  - **B (recommended)**: `"Hosted POC tenant. Every action you see is
    real."`
  - **C (minimal)**: `"Demo workspace — real actions on seeded data."`
- Adjacent test `web/components/skycot/DemoBanner.test.tsx` updated:
  drop any assertions against the literal old copy, add an assertion
  against the new copy + a `data-testid="demo-banner"` smoke that the
  banner still renders when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` and
  hides on `/onboarding/*`.
- DO NOT add `NEXT_PUBLIC_SKYCOT_DEMO_BANNER_STYLE=pill` — the pill
  is already on `/onboarding/welcome` via #2511.
- DO NOT create `web/components/skycot/DemoPill.tsx` — pill lives on
  the wizard surface only.
- DO NOT add an 8s auto-dismiss timer — the existing `localStorage`-
  backed dismiss flag (`skycot:demo-banner-dismissed`) is the
  operator-locked dismissal pattern.
- Runbook addition: a short paragraph in
  `docs/skycot/demos/v1-mission-mvp-runbook.md` near the existing
  Scene 1 pre-flight section noting the new copy + the
  `data-testid="demo-banner"` selector for the screen-capture
  selector pin.

## Shape of main on touched files

- `web/components/skycot/DemoBanner.tsx:1-100` — full file rewrite is
  out of scope. The change is a 1-line swap on the `<p>` block at
  lines 72-75.
- `web/components/skycot/DemoBanner.test.tsx` — exists, has existing
  cases for: render when `isDemoMode()` true, hide when false, hide
  on `/onboarding/*`, dismiss persists. Update copy assertion only.
- `docs/skycot/demos/v1-mission-mvp-runbook.md` — append the new copy
  note as a single bullet under the existing "Pre-warm caches"
  checklist; do not refactor adjacent sections.

## Notes for the worker

- DO NOT widen scope back to the pill / banner-style variants. The
  operator memory `feedback_no_theater_in_demo` + PR #2511 made those
  obsolete.
- DO ask the operator on PR review which copy variant (A / B / C) to
  ship — the choice is brand-voice, not technical.
- LOC budget: this rebase should be <30 LOC total across 3 files.
