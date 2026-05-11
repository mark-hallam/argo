# skycot-web

The SKYCOT product-surface SPA. Built against the locked design
language in
[`docs/build-plan/streams/adr-frontend-design-language.md`](../docs/build-plan/streams/adr-frontend-design-language.md).

## Quick start

```bash
cd web
pnpm install
pnpm dev          # http://localhost:3000
```

Open `http://localhost:3000`. The home page is the design-system
preview — every primitive, the load-bearing `ApprovalCard`, and a
live theme picker (SKYCOT light/dark + Acme tenant white-label
demo).

## Architecture in one paragraph

Next.js 15 App Router + React 19 + Tailwind v4 + shadcn-style
primitives. All visual tokens are CSS custom properties on
`:root` and `[data-theme="..."]`; the same components render
correctly across light/dark and per-tenant themes without code
changes. Theming is load-bearing for B2B adoption — see
[ADR §D3](../docs/build-plan/streams/adr-frontend-design-language.md#d3--theming-architecture-load-bearing--see-project-memory).

## Layout

```
web/
  app/                     # Next.js App Router
    layout.tsx             # Root + ThemeProvider mount
    page.tsx               # Design-system preview / smoke page
    globals.css            # @tailwind + theme variables (load-bearing)
  components/
    ui/                    # Generic primitives
      Avatar.tsx
      Button.tsx
      Card.tsx
      DataCell.tsx
      NavDrawer.tsx
      StatusPill.tsx
      Toast.tsx
    skycot/                # Skycot-domain components
      ApprovalCard.tsx     # the load-bearing artifact
      ThemePicker.tsx
      ThemeProvider.tsx    # tenant + light/dark resolution
  lib/
    utils.ts               # cn() — class-name merger
    theme/
      themes.ts            # registry + resolution
      synthesize.ts        # hex → OKLCH palette synthesizer for tenants
  stories/                 # Storybook (TODO: per-primitive stories)
  public/
```

## Theming

Three layers compose:

1. **next-themes** toggles `.dark` on `<html>` based on user pref
   or system. CSS `prefers-color-scheme` respected automatically.
2. **`ThemeProvider`** (custom, in `components/skycot/`) tracks
   tenant brand selection and writes `data-theme="<tenant>-<mode>"`
   on `<html>`.
3. **`globals.css`** defines `:root` + `.dark` + per-tenant token
   blocks. Component files use `var(--foreground)` etc., never
   hardcoded colors.

To add a new tenant theme without writing CSS:

```ts
import { synthesizeTenantCss } from "@/lib/theme/synthesize";

const css = synthesizeTenantCss({
  id: "globex",
  accentHex: "#16A34A",         // their brand green
  neutralTint: "neutral",
  radius: "rounded",
});
// inject into a <style> tag or persist server-side
```

The synthesizer takes one hex color, derives an 11-stop OKLCH
palette, and emits both `[data-theme="globex-light"]` and
`[data-theme="globex-dark"]` blocks. Built for the Operator
console's "Configure theme" surface in Track E.

## Commands

| Command | What |
|---|---|
| `pnpm dev` | Dev server with Turbopack |
| `pnpm build` | Production build |
| `pnpm typecheck` | TypeScript only, no emit |
| `pnpm lint` | ESLint + next-lint |
| `pnpm test` | Vitest unit tests |
| `pnpm test:e2e` | Playwright cross-browser |
| `pnpm storybook` | Storybook dev server (port 6006) |

## Connecting to the gateway

The dev server expects the SKYCOT gateway at
`NEXT_PUBLIC_GATEWAY_URL` (defaults to `http://127.0.0.1:8787`).
Start the gateway in another tab:

```bash
# from repo root
cargo run --bin argo -- gateway serve
```

The home page is currently a static demo; live gateway wiring
(approvals fetched from `/inbox`) lands in the Track E
`approvals-inbox` slice.

## Quality rules (inherited from the Rust side)

- **No file > 500 LOC.** If a component grows past that, split.
- **No hardcoded colors / sizes / radii.** Use `var(--...)`.
- **No new `package.json` dependency without flagging.** Bundle
  size and audit surface are explicit decisions.
- **Every primitive ships with a Storybook story** (TODO: add for
  the 8 primitives in the next slice).

## Status

- Track C (design language): IN PROGRESS as of 2026-05-11. ADR
  merged; scaffold + primitives + theming layer landed. Storybook
  stories per primitive + Playwright e2e are the next thin slice.
- Track D (wireframes): NOT STARTED.
- Track E (frontend implementation): NOT STARTED; depends on
  Track C closing.
