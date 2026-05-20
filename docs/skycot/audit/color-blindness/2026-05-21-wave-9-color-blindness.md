# Wave-9 Audit — Color-Blindness (CVD) Accessibility

**Date:** 2026-05-21
**Auditor:** Claude (Wave-9 #1)
**Scope:** SKYCOT SPA demo path — color semantics under deuteranopia, protanopia, tritanopia.
**Population impacted:** ~8% of male users (1-in-12), ~0.5% of female users.

---

## TL;DR — CVD Verdict

**Overall: PASS WITH CAVEATS.** The SPA's main demo-path components (ApprovalCard, ActivityRow, StatusPill, NotificationRow, KpiCard, TrustGradeProgress, RegressionBanner, FreezeBanner, NodeRenderer, VerdictPill, CohortCard) consistently back color with **text labels, semantic icons, or load-bearing numerals**. The token system (`oklch()`-defined `--success` / `--warning` / `--destructive` in `app/globals.css`) is a single source of truth that's well-behaved at the token level.

But there are **3 FAIL-level only-color-encoding callsites** plus **5 WEAK callsites** that need fixing before launch. The two highest-priority FAILs are in the load-bearing demo path: the `Avatar` status ring (worker status: idle/working/waiting/blocked) and the orchestration `SlaPill` (healthy/warning/breached).

---

## Methodology

Five-dimension audit per Wave-9 brief, severity scored:
- **PASS** — color + non-color backup (icon, glyph, text label, distinct shape, or load-bearing numeral)
- **WEAK** — color dominant, secondary backup present but laborious (e.g. side-context only, hover title only)
- **FAIL** — only-color critical (no non-color signal for the load-bearing semantic)

Color-vision deficiency types simulated mentally per WebAIM data:
- **Deuteranopia** (most common, ~6% males): red/green confusion, amber/yellow muddy
- **Protanopia** (~2% males): similar to deutan but red appears darker
- **Tritanopia** (rare, ~0.01%): blue/yellow confusion

---

## Dimension 1 — Severity Tiers (P0 / P1 / P2)

No P0/P1/P2 chip primitive exists in `components/ui/`. Severity-like semantics exist via:

### TicketCard priority (`components/support/TicketCard.tsx:8-13`)

```tsx
const PRIORITY_TONE = { low: "muted", normal: "info", high: "warning", urgent: "error" };
// StatusPill text content = priority literal ("low"/"normal"/"high"/"urgent")
```

**Verdict: PASS.** The pill carries the literal priority word as its text content. Color tone is reinforcing.

### CohortCard trust band (`components/skycot/CohortCard.tsx:15-21`)

```tsx
const BAND_TONE = {
  unknown: "bg-muted text-muted",
  developing: "bg-amber-100 text-amber-900",
  established: "bg-sky-100 text-sky-900",
  trusted: "bg-emerald-100 text-emerald-900",
  principal: "bg-violet-100 text-violet-900",
};
// Always rendered with `bandLabel` text ("Developing", "Trusted", etc.)
```

**Verdict: PASS.** Band label text is present.

### RegressionBanner (`components/skycot/eval/RegressionBanner.tsx`)

Icon + label combo:
- `regressed` → AlertTriangle + "Regression detected"
- `stable` → CheckCircle2 + "Stable"
- `no_baseline` → Info + "No baseline"

**Verdict: PASS.**

**Dimension 1 score: PASS.**

---

## Dimension 2 — Approve / Refuse Buttons (`ApprovalCard.tsx`)

```tsx
<Button variant="primary" onClick={() => handle("approve")}><Check /> Approve</Button>
<Button variant="secondary" onClick={() => handle("refuse")}><X /> Refuse</Button>
```

- **Approve** = primary brand purple (`--brand-primary` = `#8b5cf6`) + Check icon + "Approve" text
- **Refuse** = secondary (neutral muted bg) + X icon + "Refuse" text
- **Critical note:** Refuse is NOT a destructive red button. It's a neutral secondary. **This is actually a CVD win** — a typical green/red approve/refuse pairing would be the textbook deutan failure case, but SKYCOT avoids it.
- Post-decision state: success-tone tinted card for "Approved", muted card for "Refused" — verbs ("Approved" / "Refused") and the Clock icon for the undo countdown are visible regardless of color.

**Verdict: PASS.** Best-in-class for CVD. The decision NOT to color-code refuse as red is the right call.

---

## Dimension 3 — Status Pills (Working / Frozen / Paused / Waiting / Idle / Blocked)

### StatusPill primitive (`components/ui/StatusPill.tsx`)

```tsx
tones: success / warning / error / info / muted
```

Wrapped pills always include text children. The `dot` is the leading 6px circle — `bg-current` so it inherits the tone color.

**Verdict: PASS** when consumer passes text children (which all 24 callsites do).

### TeammateCard status (`components/skycot/TeammateCard.tsx:37-43`)

```tsx
STATUS_LABELS = { working: "Working", waiting: "Waiting on you", idle: "Idle", blocked: "Blocked", paused: "Paused" };
<StatusPill tone={tone}>{statusLabel}</StatusPill>
```

**Verdict: PASS** — labels rendered as text. Also note: aria-label on card is `${name} — ${statusLabel}` so screen-readers also catch it.

### Avatar status ring (`components/ui/Avatar.tsx:23-37`) — **FAIL**

```tsx
const statusRing = cva("absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full ring-2", {
  variants: {
    status: {
      idle: "bg-[var(--muted-foreground)]/40",
      working: "bg-[var(--info)]",      // blue
      waiting: "bg-[var(--warning)]",   // amber
      blocked: "bg-[var(--destructive)]", // red
    },
  },
});
// <span aria-hidden className={statusRing({ status })} />
```

The status ring is a **10px solid-color dot with `aria-hidden`**. No icon, no shape variation, no accessible name.
- Under deuteranopia, **amber `--warning` vs red `--destructive`** are very hard to tell apart at 10px.
- Under protanopia, blue `--info` shifts toward dark gray, possibly confusable with `idle` (gray at 40% opacity).
- A non-CVD user gets 4 distinguishable states. A CVD user effectively gets **2 distinguishable states (gray idle vs colored everything-else)**.

In the demo path: `ApprovalCard.tsx:183` passes `status="waiting"` to the Avatar. The amber dot in the corner of an approver-side card vs a red "blocked" dot is potentially load-bearing context. Today, the only signal is the dot color.

**Verdict: FAIL.** This is **only-color-critical**. Fix priority: HIGH.

**Mitigation already present:** TeammateCard does NOT use this status ring — it uses a gradient initial avatar and a separate StatusPill with text. WorkerCard (`components/ui/WorkerCard.tsx:50-58`) DOES use the avatar ring AND a co-located StatusPill so the pill rescues the ring. ApprovalCard relies ONLY on the ring.

**Dimension 3 score: PASS for pills, FAIL for the avatar ring.**

---

## Dimension 4 — Charts, Sparklines, Diffs

### KpiSparkline / MiniBar (`components/ui/charts/`)

Tone is conveyed via stroke/fill color, but **height (sparkline polyline y-position, bar rect y/h) encodes the value semantic**. Color tone is reinforcing (e.g., "this KPI is in the warning zone").

**Verdict: PASS.**

### KpiCard trend arrow (`components/skycot/worker/KpiCard.tsx:77-105`)

Trend is encoded three ways: arrow icon (ArrowUpRight / ArrowDownRight / Minus) + sign character ("+" or "−") + delta percentage. Color tone is reinforcing.

**Verdict: PASS.**

### EvalFixtureTable score-delta (`components/skycot/eval/EvalFixtureTable.tsx:46-56`)

```tsx
<td className={cn(fixture.scoreDelta < 0 ? "text-destructive" : "text-success-foreground")}>
  {fixture.scoreDelta >= 0 ? "+" : ""}{Math.round(fixture.scoreDelta * 100)} pts
</td>
```

`+` and `−` signs in text. **Verdict: PASS.**

### Audit timeline `ActivityRow` (`components/skycot/ActivityRow.tsx:89-101`)

Status tones: succeeded/failed/refused/pending → success/error/warning/info pill tones. Pill always carries text label ("Succeeded", "Failed", "Refused", "Pending").

**Verdict: PASS.**

### LiveTraceOverlay (`components/skycot/canvas/LiveTraceOverlay.tsx:58-63`)

```tsx
STATUS_DOT = {
  connecting: "bg-amber-400",
  live: "bg-emerald-500",
  polling: "bg-amber-400",   // degenerate — same color as connecting
  offline: "bg-rose-500",
};
```

Status dot color + adjacent text label ("Live" / "Connecting…" / "Polling (offline mode)" / "Offline"). The label rescues the dot.

`completed` list uses `text-emerald-700`; `failed` list uses `text-rose-700`. Each section has a HEADING ("Completed (N)" / "Failed (N)") so the section context survives.

**Verdict: PASS.**

### WorkflowDiff side-by-side (`components/skycot/WorkflowDiff.tsx:159-186`) — **WEAK**

In side-by-side mode (≥md viewport), added lines on the right pane and removed lines on the left pane get green/red row backgrounds. **No `+/−` marker is rendered** in side-by-side mode. The marker only appears in `UnifiedColumn` (mobile/narrow viewport).

Under deuteranopia, the green-50 bg and rose-50 bg at low opacity are very close. The user could tell which side is which by pane header ("from" / "to" sr-only labels) but the visual cue is purely color.

`DiffStats` summary line uses `+N -M` — that IS text-based. PASS for the summary, WEAK for the per-line gutter in side-by-side mode.

**Verdict: WEAK.** Fix priority: MEDIUM.

### ReplayDiff text token diff (`components/skycot/audit/ReplayDiff.tsx:94-117`) — **WEAK**

```tsx
// On 'original' side, 'remove' tokens: bg-destructive/12% alpha
// On 'replayed' side, 'add' tokens: bg-success/12% alpha
```

At 12% alpha, green-tint and red-tint backgrounds are nearly indistinguishable under deuteranopia. The side context partially saves this (additions are only on "Replayed" pane, removals only on "Original") but the user must mentally translate "tinted region on left pane = was removed" with no `+/−` marker or strike-through.

**Verdict: WEAK.** Fix priority: MEDIUM.

### Canvas DiffView (`components/skycot/canvas/DiffView.tsx:258-268`) — **WEAK**

```tsx
if (state === "added") return "border-emerald-300 bg-emerald-50 ...";
if (state === "removed") return "border-rose-300 bg-rose-50 ...";
if (state === "modified" || "rewired") return "border-amber-300 bg-amber-50 ...";
```

The `changedFields` array is rendered as text (e.g., "label, kind") but the per-card "added vs removed vs modified" semantic is **only visible via tint**. `data-state="added"` is on the DOM but not the visible UI.

Under deuteranopia, the user sees three similar-saturation backgrounds and must guess what they mean.

**Verdict: WEAK.** Fix priority: MEDIUM.

### Orchestration `SlaPill` (`components/skycot/orch/QueueTable.tsx:67-85`) — **FAIL**

```tsx
className={cn(
  "...",
  run.slaState === "breached" ? "border-red-300 bg-red-50 text-red-700"
    : run.slaState === "warning" ? "border-amber-300 bg-amber-50 text-amber-800"
      : "border-emerald-300 bg-emerald-50 text-emerald-700",
)}
title={`${run.latencyMs} ms of ${run.targetLatencyMs} ms`}
>
  {run.slaState === "breached" ? <AlertTriangle className="size-3.5" aria-hidden /> : null}
  <span>{pct}%</span>
</span>
```

ONLY the `breached` state has an icon (AlertTriangle). `warning` and (default = healthy) have only the percentage text. The percentage **does NOT indicate which threshold zone you're in** — 95% might be healthy under one SLA target and warning under another.

Under deuteranopia, green-50 bg vs amber-50 bg vs red-50 bg are very low-saturation washes; the difference is hard to read even for fully-sighted users at a glance. The `title` tooltip ("X ms of Y ms") helps only on hover.

**Verdict: FAIL** — the SLA tier is only-color-critical. Fix priority: HIGH.

### Cohort skill-coverage matrix (`components/skycot/CohortDetailDrawer.tsx:262-269`)

```tsx
className={cn(
  "...",
  tone === "present" && "text-emerald-700",
  tone === "missing" && "text-rose-700 bg-rose-50",
  tone === "mismatch" && "text-amber-700 bg-amber-50",
)}
>
  {tone === "present" ? "✓" : tone === "missing" ? "✗" : "≠"}
</td>
```

The text content IS a distinct glyph (`✓` / `✗` / `≠`). PASS — glyph is load-bearing.

**Dimension 4 score: PASS for charts, WEAK for diffs (3 components), FAIL for SlaPill.**

---

## Dimension 5 — Brand Gradient Text Contrast

### `BrandLoudHero` (`components/skycot/BrandLoudHero.tsx`)

```tsx
background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 50%, #ff6b6b 100%);
// White title text on top, white sub-text at 85% alpha
```

White on `#8b5cf6` (purple): ~4.42:1 contrast — passes WCAG AA for normal text.
White on `#ec4899` (pink): ~3.13:1 — passes 3:1 for large text (display ≥18pt), **FAILS for normal text**.
White on `#ff6b6b` (coral): ~2.66:1 — **FAILS even for large text**.

The hero **title** is 36px display (covered by the 3:1 rule for large text), so it scrapes through against pink but not against coral. The **sub-line** at 14px regular weight with `rgba(255,255,255,0.85)` is much worse — sub-text against the pink/coral end of the gradient will fall well below WCAG AA 4.5:1.

The **title accent gradient** (`#fff5d6 → #fde68a`, warm yellow on purple/pink/coral) is even more problematic. Under protanopia (red shifts dark), yellow text on a darkened red background is faint pale-on-darkened-pale. Under tritanopia (rare), yellow/blue confusion makes the accent gradient look interpolated into background.

### `TeammateCard` `.holo-banner` (`components/skycot/TeammateCard.tsx:227-232`)

Same gradient `#8b5cf6 → #ec4899`, white text on top for:
- Tag chip ("MARIAH" / "TEAMMATE" etc.) at 10.5px weight 800 — very small high-weight text on the gradient. WCAG accepts 14pt+ bold as "large" — at 10.5px this fails.
- LEVEL label at 9.5px weight 700 with `opacity: 0.85`
- LEVEL number at 56px weight 900 — display-class, passes.

**Verdict: WEAK to FAIL** for body-size text on the gradient. The visual register is correct (brand-loud heroes need impact) but small-text contrast is borderline.

**Dimension 5 score: WEAK.** Fix priority: MEDIUM (only affects gradient banner text, which is presentational not load-bearing — but failing WCAG is a compliance issue).

---

## Top 5 Color-Only-Encoding Callsites (Priority Order)

### 1. `Avatar.tsx:23-37` — Worker status ring (FAIL — only-color critical)

Status ring is a 10px `aria-hidden` color dot. No icon, no text, no shape. ApprovalCard (the load-bearing demo-path artifact) uses `status="waiting"` on this avatar and relies only on the amber dot. Under deuteranopia, amber vs red is borderline; protanopia makes blue+gray ambiguous.

**Fix:** Either remove the ring and rely only on the co-located StatusPill (cleanest), OR add a shape variation per status (ring shape vs solid dot vs square vs triangle for idle/working/waiting/blocked) AND set an `aria-label` so screen-readers also catch it.

### 2. `orch/QueueTable.tsx:67-85` — SlaPill tier (FAIL — only-color critical)

Three SLA states (`healthy`, `warning`, `breached`) only distinguished by background tint and number. Only `breached` has an icon. A 50% pct on healthy looks the same as 50% on warning except for the green-vs-amber tint.

**Fix:** Add an icon per state: `Check` for healthy, `Clock` for warning, `AlertTriangle` for breached. Also surface the state name in text (e.g., "95% Healthy" / "120% Breached") since users may not know the numeric threshold.

### 3. `WorkflowDiff.tsx:159-186` (side-by-side) — Diff gutter markers (WEAK)

Side-by-side mode has no `+/−` line marker. Only background tint.

**Fix:** Render the `+/−` marker even in side-by-side mode (left column gets `−` on removed lines, right gets `+` on added). UnifiedColumn already does this — copy the logic.

### 4. `audit/ReplayDiff.tsx:94-117` — Token diff (WEAK)

12% alpha tinted backgrounds for add/remove tokens. No glyph, no strike-through, no bold.

**Fix:** Add a leading `+` glyph before add tokens and `−` before remove tokens (small, monospace). OR: bold add tokens, strike-through remove tokens. The visual register stays similar but CVD users get a non-color cue.

### 5. `canvas/DiffView.tsx:258-268` — Node/edge diff state (WEAK)

Card border-and-bg tint encodes added/removed/modified. The `changedFields` text is present but doesn't say "this node was added" or "removed".

**Fix:** Add a small status icon at the top-right of each card: `Plus` for added, `Minus` for removed, `Pencil` for modified. OR: add a `data-state` badge ("ADDED" / "REMOVED" / "MODIFIED") as a small chip in the card header.

---

## Honorable mentions (WEAK but lower priority)

### `BrandLoudHero` gradient text contrast

White body-text on the purple→pink→coral gradient falls below WCAG AA 4.5:1 once you pass the purple stop. Title (36px display) gets the 3:1 large-text exemption against pink but not coral. **Fix:** Either tighten the gradient stop positions so display text always sits over the darker purple half, OR add a subtle dark text-shadow on the title for legibility, OR shorten the gradient to `#8b5cf6 → #ec4899` (drop the coral tail). The coral hex is what's pushing contrast below threshold.

### `Toast.tsx` tone variants

`success` / `warning` / `error` toasts have no leading icon. Title text usually carries the meaning ("Run cancelled" / "Couldn't load") but a CheckCircle / AlertTriangle / XCircle prefix would make tone glanceable for CVD.

### `RecentDecisionsCard` outcome chip

Outcome text ("approved" / "refused") is present in the chip, so PASS — but a Check/X icon would harden it.

---

## Top Fix (Single Recommendation)

**Adopt an "icons-not-color" rule across all status-bearing primitives.**

Every place that uses `--success` / `--warning` / `--destructive` / `--info` as the *only* signal needs a co-located icon OR text label. The token system is already in place; the violations are at the consumer level (3 FAIL, 5 WEAK).

Concrete next step: a one-page brief — `streams/mission-cvd-icons-not-color.md` — that:
1. Adds a `severity` prop to `StatusPill` with required icon mapping (`success→Check`, `warning→AlertTriangle`, `error→XCircle`, `info→Info`)
2. Refactors `Avatar.tsx` `statusRing` to use shape variation (idle = empty ring, working = solid dot, waiting = ring with center dot, blocked = solid square) and adds `aria-label`
3. Patches `SlaPill` to render its icon for all 3 states
4. Patches `WorkflowDiff` side-by-side to render `+/−` markers
5. Patches `ReplayDiff` tokens to render glyph prefixes
6. Patches `DiffView` cards with state badges

Total LOC change estimate: 80-120 lines across 6 files. No backend or schema work. No new dependencies — lucide-react icons are already imported everywhere.

Severity for v1 release: should ship before public-facing demo. The Avatar ring + SlaPill failures will be visible to any CVD user (~8% of male approvers) within 30 seconds of opening the demo.
