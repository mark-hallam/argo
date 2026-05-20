# web-tooltip-primitive — vendor a real Tooltip primitive across the SPA

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-tooltip-primitive`.
**Priority**: P2 — wave-9 a11y audit + wave-13 keyboard audit both flagged
`title="..."` native browser tooltips as the only tooltip surface: they
fail keyboard accessibility (only show on hover), have inconsistent
styling across browsers, and don't respect prefers-reduced-motion. ~30+
sites in the codebase use `title=` and need a proper primitive.
**Sizing**: ~400-500 LOC across new `<Tooltip>` primitive + accessibility
wiring + migrating ~30 callsites (mechanical sweep) + 8 unit tests + a11y E2E.

---

## Why this brief exists

Today's tooltip surface:

```bash
$ rg 'title="' web/components/ web/app/ -l | wc -l
~30
```

Every one of those is a native browser `title` attribute. Problems:
1. **Keyboard-inaccessible**: native title only fires on hover, never on
   focus. Screen-reader / keyboard users never see it.
2. **Inconsistent styling**: browser default tooltips have OS-specific
   styling (Mac vs Windows vs Chrome on Linux all look different).
3. **No theming**: native tooltip doesn't pick up the dark-mode tokens.
4. **No control over delay / positioning**: native tooltips fire on a
   browser-defined ~700ms delay with no positioning customization.
5. **Tooltips with complex content** (icons, links, formatting) are
   impossible with a `title` string.

A real tooltip primitive is the unblocker for ~30 ui-quality wins.

---

## Pre-flight verification

```bash
# 1. Confirm no tooltip primitive exists today.
ls web/components/ui/ | grep -i tooltip
rg "@radix-ui/react-tooltip" web/package.json

# 2. Count `title=` usages.
rg 'title="' web/components/ web/app/ -l | wc -l

# 3. Confirm Radix is already vendored for sibling primitives (DropdownMenu, Dialog).
rg "@radix-ui" web/package.json
```

---

## Architecture choice — Radix vs hand-rolled

**Use Radix `@radix-ui/react-tooltip`** because:
- Already in the bundle for `DropdownMenu`, `Dialog`, `Popover` — adding
  Tooltip is +4KB gzipped (the rest is shared internals).
- Radix handles keyboard focus, ARIA, portal collisions, and reduced
  motion correctly by default.
- It's the de facto pattern in the Next.js + shadcn ecosystem; future
  contributors recognize it.

Don't hand-roll. The accessibility surface area (focus management +
ARIA + portals + keyboard escape + collision detection) is too large to
reinvent for v1.

---

## Phase 1 — Vendor the primitive

### 1a. Dep install

```bash
cd web/
pnpm add @radix-ui/react-tooltip
```

Add to `web/package.json` dependencies (likely already in lockfile if any
sibling Radix primitive is in use).

### 1b. Shadcn-style wrapper

**File**: `web/components/ui/Tooltip.tsx` (new) — ~120 LOC.

The shadcn pattern: a thin wrapper that pins the brand styling and
re-exports Radix primitives so callsites import from `@/components/ui/Tooltip`
instead of the raw Radix package.

```tsx
"use client";

import * as React from "react";
import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import { cn } from "@/lib/utils";

export const TooltipProvider = TooltipPrimitive.Provider;
export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

interface TooltipContentProps
  extends React.ComponentPropsWithoutRef<typeof TooltipPrimitive.Content> {
  /** Optional tonal variant. Default is the brand-restrained slate; the
   *  inverted variant is for tooltips ON dark surfaces. */
  variant?: "default" | "inverted";
}

export const TooltipContent = React.forwardRef<
  React.ElementRef<typeof TooltipPrimitive.Content>,
  TooltipContentProps
>(({ className, sideOffset = 6, variant = "default", ...props }, ref) => (
  <TooltipPrimitive.Content
    ref={ref}
    sideOffset={sideOffset}
    className={cn(
      // Brand-restrained slate (matches dark-mode tokens).
      "z-50 overflow-hidden rounded-md px-3 py-1.5 text-xs",
      "font-medium leading-relaxed",
      "shadow-md",
      // Animations — automatically respect prefers-reduced-motion.
      "data-[state=delayed-open]:animate-in data-[state=closed]:animate-out",
      "data-[state=closed]:fade-out-0 data-[state=delayed-open]:fade-in-0",
      "data-[state=closed]:zoom-out-95 data-[state=delayed-open]:zoom-in-95",
      "data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1",
      "data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1",
      // Variants.
      variant === "default"
        && "bg-[var(--popover)] text-[var(--popover-foreground)] border border-[var(--border)]",
      variant === "inverted"
        && "bg-[var(--foreground)] text-[var(--background)]",
      className,
    )}
    {...props}
  />
));
TooltipContent.displayName = TooltipPrimitive.Content.displayName;

/** Sugar wrapper for the common case: `<HoverHint label="...">child</HoverHint>` */
export interface HoverHintProps {
  label: React.ReactNode;
  children: React.ReactElement;
  side?: "top" | "right" | "bottom" | "left";
  delayMs?: number;
}
export function HoverHint({ label, children, side = "top", delayMs = 250 }: HoverHintProps) {
  return (
    <Tooltip delayDuration={delayMs}>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side}>{label}</TooltipContent>
    </Tooltip>
  );
}
```

### 1c. Provider at the app root

**File**: `web/app/layout.tsx` (or the highest layout that wraps both
`(app)` and `(setup)` groups).

Wrap the app in `<TooltipProvider>` once at the root so every Tooltip
shares the same delay + collision detection:

```tsx
import { TooltipProvider } from "@/components/ui/Tooltip";

<TooltipProvider delayDuration={250} skipDelayDuration={100}>
  {/* existing app body */}
</TooltipProvider>
```

`delayDuration: 250ms` matches the brand's "feels instant but not jumpy"
spec from the design lock. `skipDelayDuration: 100ms` means once one
tooltip opened, sibling tooltips open near-instantly (Radix's "tooltip
group" behavior — feels right when scanning a row of icons).

---

## Phase 2 — Migrate `title="..."` callsites

A mechanical sweep — replace each native `title="..."` attribute with a
`<HoverHint label="...">` wrap.

### Pattern (before)

```tsx
<button title="Copy hash to clipboard" onClick={handleCopy}>
  <Copy size={14} />
</button>
```

### Pattern (after)

```tsx
<HoverHint label="Copy hash to clipboard">
  <button onClick={handleCopy}>
    <Copy size={14} />
  </button>
</HoverHint>
```

### Sweep targets (from the pre-flight grep)

The exact list to migrate (rg matches as of brief authoring):

1. `web/components/skycot/ActivityRow.tsx` — copy icon `title`
2. `web/components/skycot/RedactedText.tsx` — redaction sentinel `title`
3. `web/components/skycot/worker/MemoryFactsCard.tsx` — confidence pill `title`
4. `web/components/skycot/worker/RecentChangesPanel.tsx` — relative-time `title`
5. `web/components/skycot/worker/SkillsLearnedCard.tsx` — promote button `title`
6. `web/components/skycot/worker/SkillsActiveCard.tsx` — unbind button `title`
7. `web/components/skycot/worker/SkillsLibraryCard.tsx` — bind button `title`
8. `web/components/skycot/orch/QueueTable.tsx` — column header tooltips
9. `web/components/skycot/orch/ApprovalRoutingEditor.tsx` — routing-chip `title`
10. `web/components/skycot/orch/RetryBadge.tsx` — retry-policy `title`
11-30+. Remaining sites — discover via `rg 'title="' web/ --files-with-matches`

### Special-case: tooltips that mustn't intercept clicks

Some `title` attributes are on container elements where a `<HoverHint>`
wrapper would change focus / event semantics. For those, use the explicit
form:

```tsx
<Tooltip>
  <TooltipTrigger asChild>{children}</TooltipTrigger>
  <TooltipContent>Long label</TooltipContent>
</Tooltip>
```

with `asChild` so the trigger inherits the original element's tag.

---

## Phase 3 — Empty-state hints

Beyond migration, the new primitive unlocks rich tooltips that weren't
possible with `title`. Specifically:

1. **Disabled buttons explain why** — every `disabled={!canEdit}` button
   should wrap in HoverHint explaining "Editing requires admin role"
   instead of being silently dead. Sweep all `disabled=` callsites.
2. **Icon-only buttons get labels** — every icon button (Copy, Edit,
   Delete, Settings) gets a HoverHint. Already enforced for screen
   readers via `aria-label`, but visual users benefit from hover labels
   too.
3. **Truncated text** — long worker names + workflow names + skill names
   that truncate with `text-ellipsis` get HoverHint with the full string.

These are NOT mechanical — the brief flags them as follow-up
opportunities. The mechanical sweep (Phase 2) is the priority for v1.

---

## Phase 4 — Tests

**File**: `web/components/ui/Tooltip.test.tsx` (new):
1. `tooltip_opens_on_hover_after_delay`
2. `tooltip_opens_on_focus`
3. `tooltip_closes_on_escape_key`
4. `tooltip_respects_prefers_reduced_motion` (mock `matchMedia`)
5. `hover_hint_sugar_renders_correctly`
6. `inverted_variant_applies_correct_tokens`
7. `provider_delay_is_inherited_by_children`
8. `multiple_tooltips_share_skip_delay_group`

**File**: `web/tests/tooltip-accessibility.spec.ts` (new):
- E2E: Tab to a HoverHint trigger, assert tooltip becomes visible.
- E2E: Hover a HoverHint trigger, wait 250ms, assert tooltip visible.

---

## Phase 5 — Storybook (if storybook is wired)

Check: `ls web/.storybook/` — if present, add `Tooltip.stories.tsx` with:
- Default variant
- Inverted variant
- All 4 sides (top / right / bottom / left)
- With icon trigger
- With long-content trigger
- Disabled-button-with-explanation pattern

If storybook isn't wired, skip; the unit tests cover behavior.

---

## may-touch

- `web/package.json` — add `@radix-ui/react-tooltip` dep.
- `web/components/ui/Tooltip.tsx` (new) — primitive + `HoverHint` sugar.
- `web/components/ui/Tooltip.test.tsx` (new) — 8 unit tests.
- `web/app/layout.tsx` — wrap in `<TooltipProvider>`.
- ~30 component files — mechanical `title=` → `HoverHint` migration.
- `web/tests/tooltip-accessibility.spec.ts` (new) — E2E.

## must-NOT-touch

- Other Radix primitives (`DropdownMenu`, `Dialog`, `Popover`) — they have
  their own wrappers; don't touch them.
- Tailwind config — Radix tooltip animations use existing `tailwindcss-animate`
  classes.
- Native HTML `<title>` element (page metadata) — different concept.
- The hash-cell tooltip that the audit-feed brief uses — it already uses
  the `title` attribute as a stopgap; this brief replaces it with HoverHint
  IF the audit-feed brief hasn't migrated already.

---

## Acceptance criteria

- [ ] `@radix-ui/react-tooltip` is in dependencies.
- [ ] `<Tooltip>` + `<TooltipTrigger>` + `<TooltipContent>` + `<HoverHint>` + `<TooltipProvider>` exported from `@/components/ui/Tooltip`.
- [ ] App wrapped in `<TooltipProvider delayDuration={250}>` at root layout.
- [ ] Zero `title="..."` attributes remain on interactive elements in `web/components/skycot/**` and `web/app/(app)/**` (verify with `rg 'title="' web/components/skycot/ web/app/\(app\)/ | wc -l` → 0).
- [ ] Hovering for ≥250ms over any migrated trigger shows the brand-styled tooltip.
- [ ] Focusing a migrated trigger via keyboard (Tab) shows the tooltip immediately.
- [ ] Pressing Escape closes the tooltip and returns focus.
- [ ] `prefers-reduced-motion: reduce` disables the slide/fade/zoom animations (tooltip still appears, just no animation).
- [ ] 8 unit tests pass.
- [ ] E2E `tooltip-accessibility.spec.ts` passes.
- [ ] Bundle size delta ≤ +5KB gzipped (Radix tooltip is small + shares internals).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

---

## Gotchas

1. **`asChild` is load-bearing for keyboard nav**: a `<HoverHint>` that
   wraps a `<button>` should pass focus to the button, not steal it. The
   sugar wrapper uses `<TooltipTrigger asChild>{children}</TooltipTrigger>`
   which delegates focus to the child correctly.
2. **`title` attribute is NOT the same as `aria-label`**: when migrating,
   if the original element had BOTH (`<button title="X" aria-label="Y">`),
   keep `aria-label` and migrate the `title` to HoverHint. The two serve
   different audiences (hover users vs screen reader users).
3. **Tooltip with disabled buttons**: native HTML disables ALL pointer
   events on disabled buttons including hover. Radix Tooltip works
   around this by wrapping in a `<span>` — `<HoverHint>` already does
   this when child is `disabled`.
4. **Don't wrap LinkCells / NavLinks in HoverHint at top level**: the
   tooltip should pop on hover but NOT change the link's tab order or
   focus behavior. The `asChild` pattern handles this.
5. **Portal collision detection**: Radix Tooltip portals to `document.body`
   by default which can clip if the parent has `overflow: hidden`. If
   you see a tooltip getting clipped, that's a portal issue — use
   `<TooltipPortal>` (re-exported) to opt out of body-portal.
6. **`skipDelayDuration` is a TooltipProvider prop**: setting it on a
   single Tooltip does nothing. The provider-level config is global.
7. **Mocking `matchMedia` in tests**: Vitest doesn't ship `matchMedia` —
   add to `web/vitest.setup.ts`:
   ```ts
   Object.defineProperty(window, "matchMedia", {
     value: (query: string) => ({ matches: false, addEventListener: () => {}, removeEventListener: () => {} }),
   });
   ```
   so prefers-reduced-motion test can mock-toggle.

---

## References

- Radix Tooltip docs: https://www.radix-ui.com/primitives/docs/components/tooltip
- shadcn Tooltip implementation: https://ui.shadcn.com/docs/components/tooltip
- Audit: `docs/skycot/audit/a11y/2026-05-20-wave-9-a11y.md` (the keyboard-tooltip gap)
- Audit: `docs/skycot/audit/keyboard/2026-05-20-wave-13-keyboard.md`
- Sibling pattern: existing Radix wrappers — `web/components/ui/DropdownMenu.tsx`, `web/components/ui/Dialog.tsx`
