# Wave-10 Audit — Viral "Wow Moment" REVISIT (final)

**Date:** 2026-05-21
**Auditor:** Claude (wave-10 audit #4 of 4)
**Scope:** Re-evaluate the 6 viral candidates against current `origin/main`
after wave-5's recommendations have partially landed. Identify the
single screenshot for the launch tweet.
**Method:** read-only source audit of `web/app/**` + `web/components/**`
on `origin/main`. No code modified.

> Operator's brand promise (per `~/.claude/CLAUDE.md`):
> *"the UX and the visible UI must be epic, this must go viral."*

---

## What changed since wave-5

1. **PR #1900 — `/team` TeammateCard ported to C2-holographic-showcase**
   (commit `f298fb51`, merged 2026-05-21). The flat-list card became
   the operator-locked trading-card extravaganza: gradient banner per
   card, weight-900 level number, animated shine sweep on hover,
   oversized portrait avatar overlapping the banner, trait chips, the
   whole recipe. **Wave-5's "biggest miss" is closed.**
2. **PR #1898 — onboarding wrapped in F1 `WizardEditorialShell`**
   (commit `7d6e0967`, merged 2026-05-21). The 5 `/onboarding` step
   pages now carry the Audiowide hero + purple→pink gradient + warm
   radial-blob background. **`/hire-done` step 6 inherits the same
   shell** but the inner `CelebrationOverlay` is unchanged (still a
   small PartyPopper icon + `text-2xl` heading).
3. **Wave-9 fix-cycle — landing-hero bubble overlap**
   (commit `e23a4b09`, merged 2026-05-21). Added a
   `@media (max-width: 890px)` breakpoint that pins the four chat
   bubbles to the four corners (44% max-width each, non-overlapping
   grid). Below 540px the entire `.land-hero-art` is hidden so the
   subhead carries the message on mobile. **The hero is now safe to
   screenshot at every intermediate viewport.**
4. **Wave-6/Wave-7 honest-toast fixes** softened the workflow-builder
   toasts from "Workflow proposed — drafting changes" to "Preview
   build — draft not persisted." **This is the right call for buyer
   honesty** (covered in the wave-6 action-fires audit) but it
   deflates the wave-5 launch-tweet capture moment, because the
   screenshot now shows "Preview build — draft not persisted, Live
   wire ships in PR #1882" instead of the cinematic "Drafting
   workflow change…" toast.

---

## TL;DR

- **6 viral wins now ship on `origin/main`** (up from 5 in wave-5):
  - landing hero (wave-9 fix puts it back in screenshotable shape at
    every viewport)
  - **`/team` C2-holographic trading-card grid (NEW — and it is now
    the strongest frame in the codebase)**
  - workflow builder canvas + AI prompt bar (still strong, but the
    honest-toast change deflates the cinematic mid-prompt capture)
  - `/compare` 13-row matrix + gradient CTA band
  - `/skills` marketplace hero-search pattern
  - `/workers/<id>` personality card (LinkedIn for AI)
- **2 weak moments remaining**:
  - `/cohorts` still uses the generic `Card` primitive (the D3 magazine
    tile design from `project_cohorts_page_design_locked.md` has not
    been ported)
  - `/hire-done` inherits the F1 brand-loud shell but the inner
    `CelebrationOverlay` is still a tiny PartyPopper — the climactic
    emotional moment of onboarding is still smaller than a tweet
- **2 sleeper viral moments confirmed still live**:
  - action-id copy-on-hover in both the audit feed and the drill-in
    hero (`ActivityRow.tsx:103-126` + `ActionSummaryHero.tsx:130-151`)
  - ⌘K palette gradient-rose hover with corner-gradient on rows
- **The launch-tweet single frame has changed:** the new `/team`
  C2-holographic-showcase grid is now the strongest frame in the
  codebase for a single screenshot. It beats the workflow builder
  because (a) it shows 6 named teammates at once (the SKYCOT
  differentiator on one screen) and (b) the gradient banners +
  weight-900 level numbers are immediately Instagram-pretty in a way
  the builder canvas isn't.

---

## Per-screen re-scorecard

Same 3 axes as wave-5: WOW (would someone screenshot involuntarily) /
SHARE (does it teach the differentiator without context) / BRAND
(Audiowide + gradient + warm radial visible in the frame).

| # | Screen | wave-5 | **wave-10** | Δ | Notes |
|---|--------|--------|-------------|---|-------|
| 1 | `/landing` hero — 4 worker bubbles | 8/9/9 | **9/9/9** | +1 WOW | Wave-9 fix makes the hero safe to screenshot at every viewport. Previously the 700-890px intermediate band looked broken (bubbles overlapping logo). Now: 4-corner grid at intermediate, hide-art on mobile. The hero is no longer a viewport gamble. |
| 2 | `/landing` gradient compare-band | 7/7/8 | **7/7/8** | — | Unchanged. Still cinematic mid-shine. |
| 3 | `/` Approvals approve-click | 5/4/6 | **5/4/6** | — | Confetti burst from approve button is still un-shipped. Wave-5 fix outstanding. |
| 4 | **`/team` 6-worker grid** | 4/4/5 | **10/10/9** | +6/+6/+4 | **THE flip.** Wave-5's "biggest miss" became wave-10's "best frame." Brand-loud hero + 6 holographic trading cards (gradient banner, level number, shine-on-hover, trait chips, oversized portrait) in one screenshot teaches "you have hireable AI teammates with levels, traits, and dailies" in <2 seconds. No competitor (Zapier / n8n / Crew.ai / Lindy) shows this in a single frame. |
| 5 | `/workers/<id>/overview` personality card | 7/9/6 | **7/9/6** | — | Unchanged. Still distinctive (LinkedIn-for-AI framing). |
| 6 | `/workflows/<v>/<n>/build` canvas + AI prompt bar | 10/10/8 | **8/10/8** | -2 WOW | The honest-toast fix is correct for buyer trust (and aligned with the action-fires audit recommendation), but the screenshot moment is now "Preview build — draft not persisted, Live wire ships in PR #1882" instead of "Drafting workflow change…". The differentiator is still in the frame (chat + canvas), but the cinematic capture moment is gone. **For the launch tweet, this needs to be screenshotted BEFORE clicking Propose** — empty prompt bar with cursor in it + canvas behind. |
| 7 | `/audit` Etherscan-style log | 5/6/4 | **5/6/4** | — | Unchanged. Still restrained-by-design (B3-enterprise lock). Wave-5's recommendation to add a `BrandLoudHero` here was not taken — and probably shouldn't be: the audit page lock memory explicitly says "no brand-loud register on this page; gradient sparingly." |
| 8 | `/audit/action/<id>` drill-in | not scored | **6/8/5** | new | Drill-in surface has `ActionSummaryHero` with action-id copy button (`Check` flash + clipboard). Worker avatar + verb + approver/channel/timestamp + monospace action id with copy button + Replay + Export. Teaches "every action is forensically traceable" in one frame. Restrained brand (no gradient hero), but the copy-on-hover micro-interaction is genuinely viral-craft. |
| 9 | `/compare` 8/13 reveal | 9/9/7 | **9/9/7** | — | Unchanged. Still the second-best frame. The animated shine on the gradient bottom row keeps this cinematic. |
| 10 | `/skills` marketplace search | 8/7/9 | **8/7/9** | — | Unchanged. Signature hero-search pattern still distinctive. |
| 11 | `/workflows/templates` 50-card gallery | 7/6/8 | **7/6/8** | — | Unchanged. |
| 12 | ⌘K command palette | 8/8/7 | **8/8/7** | — | Unchanged. Still the best microinteraction in the codebase. |
| 13 | `/hire/done/<workerId>` celebration | not scored | **4/8/7** | new | F1 `WizardEditorialShell` (PR #1898) wraps the page with Audiowide hero + gradient + KPI strip, **BUT** the inner `CelebrationOverlay` is still a small PartyPopper icon + `text-2xl "🎉 Jamal is hired."` heading. The outer wrapper is brand-loud; the inner moment that the user actually sees is brand-quiet. This is wave-5's "hire-done is smaller than a tweet" miss — still open. |
| 14 | `/cohorts` 6-cohort grid | not scored | **4/5/5** | new | Cohort grid still uses the generic `Card`/`CardHeader`/`CardTitle`/`CardBody` primitive (`CohortCard.tsx:32-50`). The locked D3 magazine-tile layout (1 lead tile + 2 stacked + 4 compact, Bloomberg-feature aesthetic per `project_cohorts_page_design_locked.md`) has not been ported. Same loud-hero + quiet-card pattern that `/team` had before #1900. |

---

## The top 3 ship-ready viral frames (ordered by single-screenshot strength)

### 1. **`/team` — 6-card holographic trading-card grid** (NEW — the strongest frame in the codebase)

- **File:** `web/components/skycot/TeammateCard.tsx` (rendered by
  `web/app/(app)/team/page.tsx` inside `TeammateGrid`).
- **Why it wins:**
  - Six gradient-banner cards on one screen teaches "named AI
    teammates, not tools" without copy
  - Weight-900 level numbers (Mariah 5 / Priya 9 / Jamal 12 / Nori 15
    / Sam 7 / Rio 4) immediately read as "they grow with use"
  - Trait chips ("Patient · Detail-oriented · Picture-perfect
    listings · Polite") teach personality at a glance
  - The brand-loud hero band above ("YOUR AI WORKFORCE · LIVE" +
    "6 teammates, one team. · 3 working right now · 1,247 actions
    this week · saving you ~28 hours") puts a number to it
  - Animated shine sweep on hover means a 2s video clip is even
    stronger than the static screenshot
- **Capture moment:** load `/team` with 6 workers in the seed (the
  default), scroll so the hero band + top row of 3 cards are in
  frame, hover one card to trigger the shine — then screenshot.

### 2. **`/landing` hero (4 chat bubbles + Audiowide gradient H1)**

- **File:** `web/app/landing/page.tsx:811-875` (hero) +
  `web/app/landing/page.tsx:500-521` (the wave-9 viewport fix)
- **Why it still works:** unchanged 4 named bubbles (Mariah / Priya
  / Jamal / Nori) around the gradient SKYCOT logo, with the gradient
  H1 "Hire your first AI **teammate.**" + warm radial-blob
  background. **The wave-9 fix makes the screenshot safe at every
  viewport** — previously the 700-890px band looked broken.
- **Capture moment:** desktop browser, 1280px viewport (full
  4-bubble grid visible, hero art on the right).

### 3. **`/workflows/<v>/<n>/build` — canvas + AI prompt bar (with the empty prompt)**

- **File:** `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:618-647`.
- **Why it still works as launch-tweet candidate:** even with the
  toast deflated to "Preview build", the **empty-prompt + canvas
  side-by-side frame** still teaches the differentiator (chat-edit
  OR visual-edit, not either-or). No competitor shows both
  modalities in one screenshot.
- **Capture moment:** screenshot **before** clicking Propose. Place
  text cursor in the prompt bar with the placeholder visible ("Describe
  a change in plain English — e.g. 'Add a step that emails the
  seller before posting'"), gradient sparkle icon on the left,
  React Flow canvas with 3-4 gradient step-nodes behind. **Do not
  capture the toast moment** — it now says "Preview build — draft
  not persisted, Live wire ships in PR #1882," which would
  buyer-honest your way out of a launch tweet.

---

## Weak moments still open (1 PR each would fix them)

1. **`/cohorts` — magazine-tile port still un-shipped.** Same
   loud-hero + quiet-card pattern that `/team` had before #1900.
   Recommend brief `web-cohorts-d3-magazine-tile-port` — mirror of
   the #1900 brief structure, reference HTML at
   `docs/skycot/design-explorations/2026-05-18/cohorts-page/option-D3-magazine-tiles.html`
   (if it exists; otherwise authoring a new HTML reference first).
2. **`/hire/done/<workerId>` — inner CelebrationOverlay too quiet.**
   F1 outer shell is brand-loud now (#1898), but the inner card is
   `flex flex-col items-center justify-center gap-4 py-24 text-center`
   with a `size-16` PartyPopper. The literal climax of the onboarding
   journey is smaller than a tweet. Recommend brief
   `web-hire-done-celebration-confetti` — confetti overlay + weight-900
   KPI tiles ("First teammate · Hired in 4 minutes · 0 credit-card
   swipes") + Audiowide name treatment. `canvas-confetti` is 4KB.
3. **`/` approval approve-click — still missing the "ding."** Wave-5's
   confetti-burst-on-approve recommendation has not been picked up.
   The wave-1 e2e called this "the money shot." Still open.

---

## The launch-tweet single frame — REVISED

**`/team` — 6-card holographic trading-card grid + brand-loud hero**

(Wave-5 picked the workflow builder. Wave-10 changes that pick.)

Why this beats the builder for the launch tweet:

- **Self-teaching at thumbnail size.** The builder screenshot needs
  the viewer to understand what a workflow canvas is before the
  prompt-bar reads as differentiating. The team grid teaches "named
  teammates with levels, traits, status" in a half-second of
  thumbnail view.
- **Six teammates visible at once.** This is the SKYCOT moat:
  competitors sell tools, SKYCOT sells colleagues. Six named avatars
  with gradient banners + level numbers + status pills makes that
  point without copy.
- **Animated.** Shine sweeps across each card on hover. A 3-second
  video loop over the grid (cursor moving across cards, each card
  shining in turn) is the best moving thumbnail in the codebase.
- **Honest.** Wave-6's toast-honesty pass means the team grid is
  showing real data the user really has — there's nothing to soften.
  The builder screenshot now needs careful framing to avoid the
  "Preview build" toast contaminating it. The team grid has no such
  caveat.
- **The brand-loud hero is right above the grid.** One screenshot
  captures BOTH the gradient hero ("YOUR AI WORKFORCE · LIVE") AND
  the 6 holographic cards. Two viral frames in one capture.

**Capture instructions:**

1. Seed 6 workers (default seed gives you Mariah / Priya / Jamal /
   Nori / Sam / Rio with realistic levels, statuses, and bullets)
2. Open `/team` at 1440px viewport
3. Scroll so the hero band + first row of 3 cards are in frame
4. Move cursor over the middle card to trigger shine sweep
5. Screenshot at the peak of the shine animation (~0.5s into hover)

**Tweet copy candidate:** "Don't buy another tool. Hire a teammate.
Six. Each with a name, a level, and a list of what they did today.
[screenshot of /team grid]"

---

## Files touched (read only)

- `~/code/argo/web/app/landing/page.tsx`
- `~/code/argo/web/app/(app)/team/page.tsx`
- `~/code/argo/web/app/(app)/cohorts/page.tsx`
- `~/code/argo/web/app/(app)/audit/page.tsx`
- `~/code/argo/web/app/(app)/audit/action/[actionId]/page.tsx`
- `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
- `~/code/argo/web/app/compare/page.tsx`
- `~/code/argo/web/app/(hire)/hire/done/[workerId]/page.tsx`
- `~/code/argo/web/components/skycot/TeammateCard.tsx`
- `~/code/argo/web/components/skycot/BrandLoudHero.tsx`
- `~/code/argo/web/components/skycot/CohortCard.tsx`
- `~/code/argo/web/components/skycot/ActivityRow.tsx`
- `~/code/argo/web/components/skycot/audit/ActionSummaryHero.tsx`
- `~/code/argo/web/components/skycot/hire/CelebrationOverlay.tsx`
- `~/code/argo/docs/skycot/audit/viral-moments/2026-05-20-wave-5-viral-moments.md`
