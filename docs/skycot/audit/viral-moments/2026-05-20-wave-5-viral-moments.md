# Wave-5 Audit #2 — Viral "Wow Moment" Audit

**Date:** 2026-05-20
**Auditor:** Claude (wave-5 audit #2 of 4)
**Scope:** the 11 demo-journey screens — does the SKYCOT MP4 contain
screenshot-worthy moments that go viral, or is it a tasteful SaaS
demo that nobody re-shares?
**Method:** read-only source audit + curl smoke against
`localhost:3000`. No code modified.

> Operator's brand promise (per `~/.claude/CLAUDE.md`):
> *"the UX and the visible UI must be epic, this must go viral."*

This is the rubric. A demo MP4 that doesn't make a viewer stop, screenshot,
and DM it to a friend is failure for SKYCOT — not because the product
is bad, but because the bar Mark explicitly set is higher.

---

## TL;DR

- **3 clear viral wins** already shipped (landing hero, workflow builder
  prompt bar, compare page positioning band)
- **3 huge misses** where the LOCKED design promised viral and the LIVE
  implementation shipped a generic equivalent (workers grid, cohorts
  grid, hire-done celebration)
- **2 quiet sleepers** that punch above their weight (audit copy-action-id
  hover, command-palette ⌘K result preview)
- **The launch-tweet single frame:** workflow builder canvas + AI prompt
  bar with toast firing — the "two ways to edit" reveal is the only
  screenshot in this codebase that teaches the SKYCOT differentiator in
  a single frame without context

---

## Per-screen scorecard

Score 1-10 across three axes:

- **WOW** — would someone screenshot this involuntarily?
- **SHARE** — does the screenshot teach the differentiator without context?
- **BRAND** — is Audiowide + gradient + warm radial-blob visible in one frame?

| # | Screen | WOW | SHARE | BRAND | Notes |
|---|--------|-----|-------|-------|-------|
| 1 | `/landing` hero — 4 worker bubbles | 8 | 9 | 9 | The 4 bubbles + radial gradient + Audiowide H1 with gradient-text "teammate" is the single best frame in the codebase for a marketing screenshot. All 4 names (Mariah/Priya/Jamal/Nori) render. The named-teammate framing IS the SKYCOT differentiator. |
| 2 | `/landing` scroll — gradient compare-band | 7 | 7 | 8 | Animated shine sweep every 6s on the purple→pink→coral gradient band ("How do we stack up vs Zapier, n8n, Crew.ai, Lindy?"). Cinematic if caught mid-shine. |
| 3 | `/` Approvals approve-click | 5 | 4 | 6 | The "money shot" the wave-1 e2e called out is **under-delivered**. Counter ticks down quietly, no green confetti, no scale bump on the card, no big toast. `ApprovalCard` post-decision bar is a 30s undo countdown — functional, not euphoric. **One-line fix:** add a `confetti` burst from the approve button on success + scale the count pill briefly. |
| 4 | `/team` 6-worker grid | 4 | 4 | 5 | **THE BIGGEST MISS.** The LOCKED design (`docs/skycot/design-explorations/2026-05-18/workers-page/option-C2-holographic-showcase.html`) is gradient-banner trading cards with animated shine sweep + huge weight-900 level numbers — "Most viral. Best for marketing screenshots / Product Hunt." The LIVE `TeammateCard.tsx` is a flat list card with avatar + status pill + 3 bullets. Brand-loud hero above is gorgeous; below it is generic SaaS. |
| 5 | `/workers/<id>/overview` personality card | 7 | 9 | 6 | Trait chips ("Patient / Detail-oriented / Picture-perfect listings / Polite") + Great-at / Escalate-when split-pane is genuinely distinctive. Teaches "AI hires are colleagues" framing in a screenshot. Top gradient strip is the only brand touch — could pop more. |
| 6 | `/workflows/<v>/<n>/build` canvas + AI prompt bar | 10 | 10 | 8 | **THE differentiator.** Visual canvas (React Flow) + the AI prompt bar at the top with gradient sparkle icon, "Describe a change in plain English…" placeholder, and "Add a step / Remove gate" try-pills. No competitor (Zapier, n8n, Crew.ai, Lindy) shows both modalities in one frame. This is the launch tweet. |
| 7 | `/audit` Etherscan-style log | 5 | 6 | 4 | Hover-to-reveal copy icon on each `action_id` is a nice touch (`ActivityRow` line 122) but the row layout is restrained, not "Etherscan." No big "decisions are forensic" hero, no monospace ID column running edge-to-edge. **Reads as enterprise-tasteful, not viral.** The locked B3-enterprise audit design (per memory) is intentionally restrained — so this is on-spec, just not viral by design. |
| 8 | `/compare` 8/13 reveal | 9 | 9 | 7 | "If 8+ of those rows landed for you, this is your team" — bottom row is gradient-bg, animated shine, with the CTA right there. The positioning-card row across the top (5 competitors, SKYCOT highlighted with gradient text) is the second-best frame in the codebase. **One nit:** the operator brief says "8 of 13 rows are clean wins" — actual page says "If 8+ of those rows landed for you, this is your team" (subtly different). Doesn't claim a specific number. |
| 9 | `/skills` marketplace search | 8 | 7 | 9 | The signature hero-search pattern (28px gradient icon + 18px Inter input + ⌘K kbd pill + count pill + try-pills row) is brand-distinctive. "TRY" eyebrow + 6 chips ("approval routing", "candidate screening", etc.) telegraphs marketplace breadth in one frame. |
| 10 | `/workflows/templates` 50-template gallery | 7 | 6 | 8 | 50 templates exactly (counted via `grep -E "^  t\("` in `web/lib/workflow-templates.ts`). Gallery filter chips, gradient-active state, step-pill on each card. Good shot — but doesn't teach the differentiator (every workflow tool has templates). |
| 11 | ⌘K command palette | 8 | 8 | 7 | Global ⌘K with gradient corner-gradient on rows, gradient avatar initials for workers, color-coded status badges. Quick-actions ("Hire a teammate / Propose a workflow / Invite a member / Run an audit query") is the signature "this product has surface" reveal. |

---

## 5 currently-viral moments (ship as-is)

1. **`/landing` hero, 4 worker bubbles** — `web/app/landing/page.tsx:811-875`.
   The art bubbles ("Mariah: 3 listings live, 2 buyers waiting on you" /
   "Priya: 8 candidates screened today" / "Jamal: 12 tickets resolved,
   median reply 4m" / "Nori: Q2 update draft ready for review") around
   the gradient SKYCOT logo with two-tone radial blob is the single
   frame that teaches "named teammates, not tools" in one shot. The
   gradient H1 with Audiowide "teammate" + asterisk eyebrow ("v1.0
   shipped · Hiring our first 100 teams") works as a tweet image.

2. **`/workflows/<v>/<n>/build` AI prompt bar + canvas** —
   `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:589-618`.
   The top gradient prompt-bar ("Describe a change in plain English —
   e.g. 'Add a step that emails the seller before posting'") above the
   React Flow canvas of gradient-headed step nodes is the SKYCOT moat
   on a single screen. No other workflow tool shows both modalities in
   one frame. **Picks up extra viral juice if you screenshot mid-toast
   ("Proposing: 'Add a step that emails the seller before posting'").**

3. **`/compare` 13-row matrix with bottom gradient CTA band** —
   `web/app/compare/page.tsx:402-429`. Positioning cards (5 competitors,
   SKYCOT highlighted with gradient text) over a clean 13-row
   yes/partial/no matrix. The reveal frame is the bottom row: gradient
   background, animated shine, "If 8+ of those rows landed for you,
   this is your team" + immediate signup CTA.

4. **`/skills` marketplace hero search + try-pills** —
   `web/app/(app)/skills/page.tsx:326-358`. The signature search pattern
   (28px gradient icon + Inter 18px input + monospace count pill + ⌘K
   kbd) with the "TRY" row underneath teaches marketplace breadth in
   one frame. Reusable across any catalog surface (per memory:
   "signature hero-search pattern reusable").

5. **`/workers/<id>` personality card** —
   `web/components/skycot/worker/WorkerPersonalityCard.tsx`. Trait chips
   ("Patient / Detail-oriented / Picture-perfect listings / Polite") +
   Great-at / Escalate-when 2-col split + Signature behaviors at the
   bottom is the closest thing to a LinkedIn profile for an AI worker.
   Teaches "your AI hires are colleagues" framing in one screenshot.
   Top gradient strip + Heart icon is brand-consistent.

---

## 5 should-be-viral-but-aren't (with one-line fixes)

1. **`/team` workers grid is generic** —
   `web/components/skycot/TeammateCard.tsx`. The LOCKED design
   (C2-holographic-showcase) is trading-card extravaganza with
   gradient banners + huge level numbers + animated shine sweep on
   hover. The LIVE card is a flat list card with avatar + status
   pill + 3 bullets. **Fix:** swap `TeammateCard.tsx` body to render
   the C2 trading-card HTML pattern — gradient banner per cohort,
   huge level number top-right, on-hover shine animation. The
   reference HTML already exists at
   `docs/skycot/design-explorations/2026-05-18/workers-page/option-C2-holographic-showcase.html`
   so this is a one-day port, not a redesign.

2. **`/cohorts` grid uses generic Card primitive, not magazine-tile
   layout** — `web/components/skycot/CohortCard.tsx` is plain
   `<Card><CardHeader><CardTitle>` + `<CardBody>` with a numbers
   grid. The LOCKED design (D3 magazine tiles, per memory) is
   "1 lead tile + 2 stacked + 4 compact" with editorial Bloomberg-feature
   aesthetic. **Fix:** asymmetric grid layout + larger lead tile with
   gradient tab, smaller stacked + compact below. Reference: cohorts
   design-lock memory + the operator's "brand-loud-editorial register"
   note.

3. **Hire-done celebration is too quiet** —
   `web/components/skycot/hire/CelebrationOverlay.tsx`. A small
   PartyPopper icon in a circle + `text-2xl` heading is the entire
   payoff for "you just hired your first AI teammate." That's the
   single biggest emotional moment in the onboarding journey and it's
   smaller than a tweet. **Fix:** wrap the celebration in
   `<BrandLoudHero>` with confetti overlay, Audiowide gradient name,
   weight-900 KPI tiles ("First teammate · Hired in 4 minutes · 0
   credit-card swipes"). The brand-loud hero recipe is already in
   `web/components/skycot/BrandLoudHero.tsx` — just import it.

4. **Approval approve-click is missing the "ding"** —
   `web/components/skycot/ApprovalCard.tsx:295-303`. Post-approve, the
   card shifts to a 30s undo countdown bar. Functional, not euphoric.
   The wave-1 e2e called this "the money shot" — it should be visceral.
   **Fix:** confetti burst from the Approve button (`canvas-confetti`
   is a 4KB lib), scale-pulse the count pill in the header (already a
   `<StatusPill>` showing pending count), and brighten the
   `BrandLoudHero` KPI value briefly. The hero already exists; only
   the burst + pulse is missing.

5. **`/audit` log doesn't read as "we know everything"** —
   `web/components/skycot/audit/AuditDayGroup.tsx` + `ActivityRow.tsx`.
   The locked B3-enterprise design (per memory) deliberately picked
   restrained-Inter + Etherscan-table aesthetic. But the live page is
   so restrained it doesn't communicate "forensic-grade audit chain"
   in a screenshot. **Fix:** add a `BrandLoudHero` variant for `/audit`
   with eyebrow "AUDIT CHAIN · LIVE", weight-900 KPI tile showing total
   actions (e.g. "12,847 actions audited · 0 unaccounted") + the
   pulsing live dot. The hero teaches the differentiator; the
   restrained log below proves it.

---

## 2 unexpected viral sleepers

1. **Action ID copy-on-hover in audit feed** —
   `web/components/skycot/ActivityRow.tsx:103-126`. Hovering an audit
   row reveals a copy icon next to the action ID, click flashes a
   Check + copies the ID to clipboard. It's tiny but it's the kind of
   craft-detail that gets screenshotted with the caption "this is what
   an enterprise audit log should feel like." Doesn't need a fix —
   just ensure the demo MP4 hovers a row for 2s so the copy icon
   appears.

2. **⌘K palette gradient-rose hover state** —
   `web/components/skycot/CommandPalette.tsx:746-749`. The active
   row's `linear-gradient(135deg, rgba(139, 92, 246, 0.1),
   rgba(236, 72, 153, 0.08))` background with a subtle 1px gradient
   inset shadow is the most polished microinteraction in the
   codebase. Worth a 1-second demo pause on the open palette frame
   with the arrow key cycling through results.

---

## The launch-tweet single frame

**`/workflows/sales/qualify_lead/build` — the visual builder mid-prompt.**

Why this beats the landing hero for the launch tweet:

- Teaches the differentiator in one frame (chat-edit OR visual-edit,
  not either-or)
- Demonstrates the product is real (React Flow canvas with gradient
  step-nodes, mini-map, side panel, gradient floating Add-step button)
- Cinematic — animated edges between nodes, gradient on the prompt
  icon, gradient on Save+publish CTA
- Wide aspect — fits a Twitter card without margins
- Reads in <2 seconds: "this is a workflow builder + 'describe in
  English'" — beats every competitor's screenshot in the same frame

Capture moment: type "Add a step that emails the seller before posting"
into the prompt bar, click Propose, screenshot the loading toast
("Proposing: 'Add a step that emails the seller before posting'…
Drafting workflow change…") with the canvas behind it.

**Tweet copy candidate:** "Don't buy another tool. Hire a teammate.
Edit them by chatting in plain English. Or with a canvas, if you're
old-school. [screenshot]"

---

## Closing note for the conductor

The viral problem is not "we don't have viral moments." The viral
problem is **3 of the loudest-promised pages ship the brief-restrained
implementation instead of the brief-loud implementation.** Specifically:

- `/team` has the brand-loud hero but a generic card grid below it.
  Loud-then-quiet is more frustrating to viewers than uniformly-quiet.
- `/cohorts` is the same pattern (loud hero + restrained card grid).
- The hire-done overlay is the literal climax of the onboarding
  journey and it's smaller than a tweet.

Fixing these three is 2-3 days of front-end work that turns three
"should-be-viral" frames into three "are-viral" frames. The single
strongest argument for prioritizing this: the operator's brand
promise IS the bar, and "epic and viral" is not the same as "tasteful
and on-brand."

---

**Files touched (read only):**

- `~/code/argo/web/app/landing/page.tsx`
- `~/code/argo/web/app/(app)/page.tsx`
- `~/code/argo/web/app/(app)/team/page.tsx`
- `~/code/argo/web/app/(app)/audit/page.tsx`
- `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
- `~/code/argo/web/app/(app)/workflows/templates/page.tsx`
- `~/code/argo/web/app/(app)/skills/page.tsx`
- `~/code/argo/web/app/(app)/cohorts/page.tsx`
- `~/code/argo/web/app/(app)/workers/[workerId]/overview/page.tsx`
- `~/code/argo/web/app/(app)/audit/action/[actionId]/page.tsx`
- `~/code/argo/web/app/compare/page.tsx`
- `~/code/argo/web/components/skycot/BrandLoudHero.tsx`
- `~/code/argo/web/components/skycot/TeammateCard.tsx`
- `~/code/argo/web/components/skycot/CohortCard.tsx`
- `~/code/argo/web/components/skycot/ApprovalCard.tsx`
- `~/code/argo/web/components/skycot/ActivityRow.tsx`
- `~/code/argo/web/components/skycot/hire/CelebrationOverlay.tsx`
- `~/code/argo/web/components/skycot/CelebrationBanner.tsx`
- `~/code/argo/web/components/skycot/CommandPalette.tsx`
- `~/code/argo/web/components/skycot/worker/WorkerPersonalityCard.tsx`
- `~/code/argo/web/components/skycot/audit/AuditDayGroup.tsx`
- `~/code/argo/web/components/skycot/audit/ActionSummaryHero.tsx`
- `~/code/argo/web/lib/workflow-templates.ts`
- `~/code/argo/docs/skycot/design-explorations/2026-05-18/workers-page/option-C2-holographic-showcase.html`
