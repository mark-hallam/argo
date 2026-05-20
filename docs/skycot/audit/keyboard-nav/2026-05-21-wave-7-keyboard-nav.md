# Wave-7 Keyboard-Only Navigation Audit — SKYCOT SPA

**Auditor:** Wave-7 Agent #2 (keyboard-only navigation)
**Date:** 2026-05-21
**Scope:** Demo path — every surface a procurement reviewer touches with Tab / Shift+Tab / Enter / Esc / arrow keys, no mouse
**Standard:** WCAG 2.1 AA — SC 2.1.1 (Keyboard), 2.1.2 (No keyboard trap), 2.4.3 (Focus order), 2.4.7 (Focus visible)
**Tooling notes:** static read of every `app/` route and `components/` widget. No live keyboard run in Chromium — findings are derived from source review of focus management, semantic tags, role/aria coverage, focus-visible styling, and known Radix-primitive guarantees.

---

## Executive summary

**Average keyboard-nav score across 8 demo-path surfaces: 7.6 / 10.**

The SPA inherits a solid keyboard foundation from disciplined primitive choices:

- **All popovers + dropdowns + dialogs use Radix UI** (`@radix-ui/react-popover`, `react-dropdown-menu`, `react-dialog`). Out of the box that gives the SPA: Esc-to-close, click-outside-to-close, focus trap inside the panel, focus return to the trigger on close, arrow-key navigation within menus, type-ahead within menus.
- **Shared `<Button>` primitive** in `web/components/ui/Button.tsx:9-14` declares `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2` on every variant. Every place the team used `<Button>` (~80% of action surfaces) is automatically OK.
- **Global safety net** in `web/app/globals.css:276-279` — `:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }`. This catches the few places designers wrote bespoke buttons without a focus ring (the landing page hero CTAs, the workflow builder header buttons).
- **Native HTML form elements** (`<input>`, `<select>`, `<textarea>`, `<a href>`, `<button type="submit">`) used everywhere for the auth + onboarding flows. Native browser keyboard works for free.

The procurement reviewer **can complete the demo without a mouse**, but with five friction points that will trigger discussion. None is a binary fail of WCAG 2.1.1 — every required action can be reached — but #1 and #2 below are likely to be flagged by an accessibility auditor as "operable but not equivalent to mouse experience."

---

## Top-five fails

### 1. Landing "For" verticals dropdown is hover-only (`web/app/landing/page.tsx:234-237`)

```css
.land-for-wrap:hover .land-for-menu,
.land-for-wrap:focus-within .land-for-menu {
  display: grid;
}
```

The dropdown shows on `:hover` and `:focus-within`. The `:focus-within` selector means tabbing into the trigger button keeps the menu visible while focus is anywhere inside — but the trigger itself is not styled as a button with `aria-expanded`, doesn't toggle the menu on Enter, and there's no Esc-to-close. A keyboard user gets the menu open by tabbing onto the "For" button, but it stays open after they tab past — and they can't dismiss it without committing to a vertical or shift-tabbing back out. The menu items are `<Link>` so Enter activates them, but the structural pattern is wrong.

**Failed criteria:** SC 2.1.1 partial (operable, but in a different mode than mouse users), SC 4.1.2 (the trigger has `aria-haspopup="true"` but no `aria-expanded` to reflect open state — screen readers can't tell whether the menu is open).

**Fix:** convert to `@radix-ui/react-dropdown-menu` (already used elsewhere in the SPA), kill the CSS-only `:hover` open. ~15 lines diff.

### 2. Workflow builder ReactFlow canvas — selected node not keyboard-activatable (`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:639-660`)

The React Flow canvas is the hardest surface in the SPA. xyflow/react v12 *does* support keyboard out of the box — Tab walks between nodes, Backspace deletes the selected node, arrow keys move the selected node. However:

- `onNodeClick` is the only handler bound. There's **no `onNodeKeyDown`** or equivalent. So a keyboard user can focus a node (Tab) and see the focus ring (React Flow draws it), but pressing Enter does **not** open the right-hand config panel — they have to click. The right-hand panel is the entire point of selecting a node in the build view.
- The custom `SkycotStepNode` (`page.tsx:134-234`) has no `aria-label`, no `role`, and no `tabIndex` override — though `aria-label` is built into React Flow's node wrapper.
- The "Add step" button (`page.tsx:661-663`) is a real `<button>` and gets the global focus-visible ring, but the AI prompt bar's "Add a step" / "Remove gate" suggestion chips (`page.tsx:629-634`) have neither hover state nor focus-visible styling — they inherit the global outline, but the visual lift is invisible against the gradient background.
- `Test run` and `Save + publish` header buttons (`page.tsx:593-602`) are real `<button>` elements — keyboard-OK.

**Failed criteria:** SC 2.1.1 partial — node selection is reachable by keyboard but the resulting "Edit step" action requires a click that isn't equivalent via keyboard. A procurement reviewer who tries to demo the brand-claim "edit workflows in chat" via the visual canvas will be stuck.

**Fix:** add `onNodeKeyDown` to the React Flow component, route Enter/Space to `setSelectedNode(node)`. ~5 lines.

### 3. No skip-to-content link in root layout (`web/app/layout.tsx`)

The SPA has 12+ navigation items in the left sidebar (workers / team / approvals / audit / skills / workflows / cohorts / settings / billing / etc., plus the brand link, plus the workspace switcher footer). A keyboard user landing on `/audit` who wants to drill into the first audit row has to tab through every sidebar link first — 13 Tab presses before they reach the search bar, then another 4 before they reach the first audit row.

A standard "Skip to main content" link as the first focusable element on the page is the universal fix. Doesn't yet exist.

**Failed criteria:** SC 2.4.1 (Bypass Blocks) — explicit WCAG A-level criterion, not even AA.

**Fix:** add a visually-hidden-until-focused `<a href="#main-content">Skip to main content</a>` at the top of `RootLayout`, and an `id="main-content"` on the `<main>` element in `AppShell`. ~10 lines.

### 4. Workflow builder AI prompt "suggest" chips have no focus styling (`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:629-634`)

```tsx
<button type="button" className="builder-prompt-suggest" onClick={() => setPromptText("Add a step…")}>
  Add a step
</button>
```

The chips are real `<button>` elements (good), but the `.builder-prompt-suggest` CSS class declares only `:hover` (`page.tsx:500`) — no `:focus-visible`. They get the global 2px outline from `globals.css`, but it's a thin purple line on a thin purple chip border on a pale-purple background, **functionally invisible**. Same issue affects:

- Landing-page footer nav links (`web/app/landing/page.tsx:1136-1166`) — `<Link>` with no focus class. Global outline applies, but contrast is poor on the `--bg: #fafaff` page background.
- Landing-page hero "For" trigger (`page.tsx:861-863`) — same.
- Cohort/skill page hero search chips (per locked design memory) — likely same pattern. (Not read in full this audit.)

**Failed criteria:** SC 2.4.7 (Focus Visible) — present but contrast-inadequate is a partial fail.

**Fix:** add `focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2` to every bespoke button class in the landing + workflow-builder + workflows-index + skills hero pattern. ~20 lines across 5 files.

### 5. Landing page "Watch a 90-second demo" CTA is an `<a href="#how">` anchor (`web/app/landing/page.tsx:916-918`)

The hero's secondary CTA is an in-page anchor that jumps to `#how`. That works for keyboard navigation — Enter follows the anchor, focus moves to the heading. But the user has no way to know they've moved 4 sections down except by visually noticing the page jumped. There's no `tabindex="-1"` on the heading to receive focus, so the next Tab press goes to the **next link after the CTA in DOM order**, not the first interactive element of the "How it works" section.

This isn't an a11y blocker — but it's the kind of dead-end a procurement reviewer will notice ("I pressed Tab, where am I?"). Anchor-jumps without target-focus is a known WCAG 2.4.3 friction.

**Failed criteria:** SC 2.4.3 (Focus Order) — partial. The focus order is logical but the anchor-jump breaks the user's mental model.

**Fix:** add `tabindex="-1"` to `#how` heading and call `.focus()` on it in a small `onClick` handler. ~3 lines.

---

## Per-page scores

### /landing — **7 / 10**
| Check | Result |
|---|---|
| Tab order logical | Yes — nav → hero CTAs → trust strip → How-it-works → Features → Social proof → Compare → Final CTA → Footer |
| Focus visible | Partial — hero primary CTA + secondary CTA have no `:focus-visible` rule, rely on global 2px outline; chip + nav links same. Contrast adequate but not designed-for. |
| No mouse-only | Partial — "For" dropdown (Fail #1 above) opens on hover, no Esc-to-close, no `aria-expanded`. Tabbing through reveals it via `:focus-within` but the dismiss model is broken. |
| Modals/popovers Esc-closes | N/A on landing |
| Skip-to-content | **No** (Fail #3) |
| Score | **7 / 10** |

### /auth/signup — **9 / 10**
Real `<form>`, real `<input>` with `autoComplete="email"`, real `<button type="submit">`, Enter submits, autofocus on email, ⌘+Enter shortcut visible as `<span class="kbd">↵</span>`. SSO buttons are real `<button>` elements with `disabled` attribute. The only mark-down: no skip-link, and the SSO buttons' focus rings on the gradient-highlight variant are thin against the lavender border (Fail #4 register).

### /auth/signin — **9 / 10**
Same patterns as signup. Adds the SSO discovery probe — UI swaps "Continue" CTA for "Continue with Google" without losing focus or breaking Tab order. Conditional rendering is keyboard-friendly because the focused button is the same DOM element via the same form.

### / (Approvals inbox — load-bearing page) — **8 / 10**
| Check | Result |
|---|---|
| ApprovalCard | Excellent — semantic `<details>/<summary>` for draft, real Approve/Refuse/Why? `<button>`s with focus-visible, aria-expanded on Why?, aria-live="polite" on undo countdown. |
| FilterPopover | Excellent — Radix Popover, role="checkbox" + aria-checked on each option, focus-visible. Esc closes. |
| NotificationsPopover (Bell) | Excellent — Radix Popover, lazy-loaded, focus return. |
| Search input | Real `<input type="search">`, aria-label. |
| BatchActionBar / Day group headers | Real buttons, focus-visible. |
| Hire teammate button | Real Button primitive. |
| **Friction** | Sticky page header has 4 trailing controls (Filter / Notifications / Hire) and then jumps to search input — Tab order is OK but no skip-link to leap past the day-section headers to the first card. |

### /team — **8 / 10**
TeammateCard is `<div role="group" tabIndex={0}>` with `onKeyDown` that handles Enter + Space and a fallback `:focus-visible` box-shadow ring (`TeammateCard.tsx:212-218`). The kebab "context menu" is a real `<button>` via Radix DropdownMenu — keyboard-OK. Two issues:
1. The whole card is clickable; the "Open profile" link inside `e.stopPropagation()`s its click — but a keyboard user pressing Enter on the focused card vs. the focused link gets the same result (both push to `/workers/<id>`). OK.
2. The pause/resume confirmation is `<AlertDialog>` (Radix) — focus trap + Esc + focus return all work.
3. **Friction:** the trait chips and bullets are not in tab order — but they're descriptive content, not actionable, so that's correct.

### /workers/wkr_01J0MARIAH/overview — **8 / 10**
WorkerLayoutShell + WorkerHero + WorkerTabs all clean. `WorkerTabs` (`web/components/skycot/WorkerTabs.tsx`) is the tab pattern done correctly: `<nav aria-label>` → `<ul role="tablist">` → `<li><Link role="tab" aria-selected aria-current>`. The Link gets focus-visible ring. The overflow menu (kebab) uses Radix DropdownMenu — keyboard-OK. The personality card, KPI cards, and recent-changes panel are read-only content, so no tab-order concerns. Activity/Memory/Skills/Trust/Settings sub-tabs share the same WorkerTabs pattern.

### /workflows + /workflows/sales/qualify_lead/build — **5 / 10**
**The hardest page in the SPA and the lowest score.**

The `/workflows` index page is OK — Link cards, real `<button>` for "Propose new", real `<input>` for search, real `<button>` chips for try-suggestions. ~8/10 in isolation.

But the `/workflows/<v>/<n>/build` page is where the demo claims SKYCOT's "class-leading" visual workflow editor. Without a mouse:
- The header buttons (Back, Test run, Save + publish) work.
- The AI-prompt bar's `<input>` + Send `<button>` + suggest chips work.
- The React Flow canvas (`page.tsx:639-660`) lets a keyboard user **walk between nodes** (Tab) and **see which is focused** (React Flow draws a ring) — but pressing Enter does nothing. There is **no `onNodeKeyDown` handler** wiring keyboard activation to `setSelectedNode`. Selecting a node requires a click. (Fail #2 above.)
- Once a node is selected via click, the right-side `aside.builder-panel` opens — keyboard-OK from there.
- The "Add step" floating button works.
- **Score: 5/10** — keyboard reviewer can navigate to most controls, but the canvas's core interaction (select node → see config → edit step) requires a mouse.

### /audit + /audit/action/<id> — **9 / 10**
| Check | Result |
|---|---|
| AuditPageHeader | Native `<input type="search">`, native `<select>` (8 range presets), real `<button>` for Filter / Export. All keyboard-OK. |
| AuditFilterPopover | Radix Popover. Multi-select dimensions exposed as `aria-checked`/`role=checkbox` `<button>`s with focus-visible. |
| AuditDayGroup | Renders ActivityRow (`web/components/skycot/ActivityRow.tsx`) — `<Link href>` with `focus-visible:outline-none focus-visible:ring-2`. Best-in-class. |
| ActivityRow children | Action-id copy is a real `<button>` with `e.stopPropagation()` so Enter on the row navigates, Tab + Enter on the copy button copies. |
| Load more | Real `<button>` with focus-visible. |
| EvidenceBundleExporter | Radix Dialog. Focus trap + Esc + focus return. |
| **Score** | **9 / 10** — only mark-down is no skip-link. |

---

## Summary scoreboard

| Surface | Score | Headline issue |
|---|---|---|
| /landing | 7 | "For" dropdown is hover-only with broken dismiss |
| /auth/signup | 9 | Skip-link only |
| /auth/signin | 9 | Skip-link only |
| / (Approvals) | 8 | No skip past header to first card |
| /team | 8 | None — well-built keyboard-clickable card |
| /workers/<id>/overview | 8 | Skip-link only |
| /workflows/<v>/<n>/build | 5 | React Flow node selection requires mouse |
| /audit | 9 | Skip-link only |
| **Mean** | **7.6** | |

---

## Highest-impact single fix

**Add `onNodeKeyDown` to the workflow builder ReactFlow component.** ~5 lines:

```tsx
onNodeKeyDown={(e, node) => {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    setSelectedNode(node);
    setTriggerConfigOpen(false);
  }
}}
```

That single change converts the highest-friction page in the demo path from 5/10 to 8/10 and removes the only place in the SPA where a procurement reviewer cannot reach a feature without a mouse. Skip-to-content is the second-highest impact (helps 7 of 8 surfaces by 0.5–1 point each) but isn't a feature unlock — it's an ergonomic improvement.

---

## Can a procurement reviewer do the demo without a mouse?

**Yes — with two caveats they will name:**

1. They will not be able to demonstrate the workflow builder's node-config panel without a mouse (or without arrow-key + Enter to dispatch a synthetic click, which is not a standard a11y pattern). This is the one place where the keyboard story breaks down for a feature that's load-bearing for the brand narrative.
2. They will note the absence of a skip-to-content link as a WCAG 2.4.1 A-level fail in their report. This is a 10-line fix.

Everything else — login, approvals, team browse, worker detail, audit drill-in, settings — is reachable, operable, and dismissable via Tab / Shift+Tab / Enter / Esc. The reviewer will sign off, but with a punch-list of five small changes attached.
