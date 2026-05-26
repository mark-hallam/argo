# Round 88 — PR #2418 DemoBanner sidebar overlap — PRE-MERGE PASS

- **Date**: 2026-05-26
- **Round**: 88
- **Pillar / Claim**: Demo-recording polish (UX-AUDIT-T2309MEL); v1.0 demo MP4 hygiene.
- **Verdict**: 🟢 **PRE-MERGE PASS** (P2 — visible-on-recording)
- **PR head SHA**: `67824088da4ce7e89d38ba19150dc0a99ca4c133`
- **Size**: +122 / −1 across 2 files (web-only).

## Setup

- Worktree at `/tmp/eval-r88-wt/pr-2418`, detached at PR head.
- `pnpm install --frozen-lockfile` succeeded (7.2s).
- Read production diff at [web/components/skycot/AppShell.tsx](web/components/skycot/AppShell.tsx) and the playwright spec at [web/tests/e2e/app-shell-demo-banner-sidebar-no-overlap.spec.ts](web/tests/e2e/app-shell-demo-banner-sidebar-no-overlap.spec.ts).

## Observed

**Production change** — Single-line move: `<DemoBanner />` was a sibling of `<main>` at the root of `AppShell`; the PR moves it inside `<main className="md:pl-64 focus:outline-none">`. The banner now inherits the 256px content-column offset at `>=768px` viewports (mobile is unaffected because `md:pl-64` only applies above the `md:` breakpoint, and mobile uses a bottom-sheet drawer, not a fixed left rail).

**Test** — 3 playwright specs covering:

1. Desktop (1440×900): structural assertion via `node.closest("main#main")`, geometric assertion via `box.x >= 256`, copy assertion via `toContainText("Demo mode")`.
2. Dismiss-button persistence: clicks the dismiss test-id, asserts banner unmounts, asserts `localStorage["skycot:demo-banner-dismissed"] === "1"`.
3. Mobile (390×844): asserts `box.x < 10` since the sidebar is collapsed on mobile.

Spec self-skips if `NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1"` — clean pattern matching `onboarding-demo-mode-step-2-continue.spec.ts`.

**Vitest** — Same pre-existing 6 failures as PR #2413 (3 test files, all on origin/main). The PR introduces zero new failures.

**Typecheck** — Same pre-existing 2 errors (4th-round and 3rd-round unclaimed carries). Neither touches `AppShell.tsx` or `DemoBanner.tsx`.

## Anti-theater check

Dispositive on three independent axes:

1. **Structural** — `closest("main#main")` is dom-walking; the assertion can only pass if `DemoBanner` is rendered as a descendant of `<main id="main">`. The production change is exactly this — moving the element inside `<main>`.
2. **Geometric** — `boundingBox().x >= 256` is what the user actually sees, measured by the real layout engine. Could only pass if the banner inherits `pl-64` from its parent. If the move had been to a non-padded parent, this assertion would fail.
3. **Mobile inverse** — `box.x < 10` on mobile asserts the OPPOSITE behaviour (banner spans full width when the sidebar is hidden). A test "rewritten to pass" couldn't simultaneously satisfy both desktop AND mobile assertions unless the production code actually nests correctly under the `md:` breakpoint.

Cross-checked: the dismiss-flow test is a real regression guard — it would have failed the previous behaviour because `<DemoBanner>` no longer renders at the root and the dismiss state has to round-trip through `localStorage` correctly.

**84th anti-theater verification this week.**

## Recommendation

🟢 **P2 MERGE** under operator-locked permanent `feedback_web_only_merge_authorization_2026_05_26` lane (web-only ≤200 LOC ≤2 files). Lower priority than #2413 because the overlap is cosmetic — but it IS visible on the v1.0 demo recording, so deserves to ship before the operator hits record.

## Gap / follow-up brief candidates

None. The fix is complete and well-tested.
