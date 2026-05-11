# ADR — Frontend design language for SKYCOT SPA

**Status**: ACCEPTED 2026-05-11.
**Authored against**: origin/main `70e4b020` on `2026-05-11`.
**Track**: PRODUCT_ROADMAP.md Track C (design language).
**Decision-maker**: operator (locked-in 2026-05-11).

## Context

PRODUCT_ROADMAP.md Tracks D (wireframes) and E (frontend
implementation) need a locked design language and stack. Without
it, every parallel brief in Track E would re-litigate basic
decisions (which component library, what's a button's radius,
how does dark mode work) — producing 30 disconnected
components instead of a coherent product.

This ADR is the single locking decision for Track C. Once
merged, downstream briefs reference *this document*, not
component-by-component re-choices.

The SKYCOT brand itself is **deferred** — see PRODUCT_ROADMAP.md
"Open questions for the operator." The architecture documented
here separates *what the system supports* (theming primitives,
swap-points) from *what we picked for SKYCOT* (the default theme
values). Brand lock-in is a single-day swap when the operator
is ready.

## Decisions

### D1 — Stack

| Layer | Choice | Why |
|---|---|---|
| Framework | Next.js 15 App Router + React 19 + TypeScript | Mature ecosystem; SSR for marketing/SEO surfaces later; server components for low-interactivity pages (audit timeline); client components for high-interactivity ones (approval card, hire flow) |
| Styling | Tailwind CSS v4 | CSS-first, fast, tokenizable; v4 reads CSS custom properties natively |
| Components | shadcn/ui (Radix primitives, copy-paste) | We own the code; accessible by default; no dependency lock-in |
| Server state | TanStack Query | Cache management, optimistic UI, retry — all of which the approval card needs |
| Client state | Zustand (only where state is genuinely client-local) | Smaller than Redux; most state belongs in URL or server |
| Forms | react-hook-form + Zod | Standard shadcn pairing; Zod schemas double as API contracts |
| Icons | Lucide React | Default for shadcn; consistent line weight; ~1400 icons |
| Auth | NextAuth v5 (Auth.js) bridged to `ARGO_GATEWAY_BEARER_TOKEN` | Session/SSR glue on top of the gateway's existing bearer auth |
| API client | Typed `fetch` wrapper for v0; OpenAPI codegen when gateway exposes a spec | Don't block on codegen tooling |
| Motion | Tailwind transitions baseline; Motion (formerly Framer Motion) for orchestrated cases (approval-card slide + undo countdown) only | Avoid motion-library overhead for simple fades |
| Testing | Vitest + Testing Library + Playwright | Vitest fast; Playwright cross-browser e2e |
| Component dev | Storybook 8 (CSF3) | Mandatory for design system; every primitive ships with stories |
| Package manager | pnpm | Fast, less disk than npm, monorepo-friendly |
| Deployment | Self-hostable Docker; Vercel-compatible bonus | Mission invariant: platform must be ownable end-to-end |

### D2 — Project location

`web/` at the repo root. NOT inside `crates/`. Reasoning: the SPA
is a separate process (Node) consuming the gateway HTTP API. The
Rust workspace stays clean. CI gates for `web/` are separate.

### D3 — Theming architecture (load-bearing — see project memory)

The SPA supports **per-tenant themes**, not just light/dark. This
is a hard requirement (operator amendment 2026-05-11). Architecture:

**All visual tokens are CSS custom properties** on `:root`,
overridable via `[data-theme="<name>"]` selectors. No hardcoded
colors in component files. The token contract:

```css
:root {
  /* surfaces */
  --background:        ...;
  --foreground:        ...;
  --card:              ...;
  --card-foreground:   ...;
  --popover:           ...;
  --popover-foreground:...;

  /* primary action */
  --primary:           ...;
  --primary-foreground:...;

  /* secondary / muted */
  --secondary:           ...;
  --secondary-foreground:...;
  --muted:               ...;
  --muted-foreground:    ...;
  --accent:              ...;
  --accent-foreground:   ...;

  /* semantic */
  --destructive:           ...;
  --destructive-foreground:...;
  --success:               ...;
  --warning:               ...;
  --info:                  ...;

  /* chrome */
  --border:           ...;
  --input:            ...;
  --ring:             ...;

  /* shape + motion */
  --radius-sm:        ...;
  --radius:           ...;
  --radius-lg:        ...;
  --duration-fast:    150ms;
  --duration-base:    200ms;
  --duration-slow:    300ms;

  /* type */
  --font-sans:        "Inter", system-ui, sans-serif;
  --font-mono:        "Geist Mono", ui-monospace, monospace;
}
```

**Shipped themes day 1:**
- `:root` — light default (skycot-light)
- `.dark` — dark default (skycot-dark, system-preference auto)
- `[data-theme="skycot-light"]` — explicit alias for the default
- `[data-theme="skycot-dark"]` — explicit alias for the default dark

**Per-tenant custom themes:**
A tenant can register a theme via gateway config. Minimum
customizable surface:
- Accent color (single hue; we derive the 11-stop palette)
- Neutral tint (cool / neutral / warm — we map to a slate / zinc / stone palette)
- Radius scale (sharp / default / rounded)
- Logo asset (SVG preferred; PNG fallback)
- Font choice (curated set: Inter, Geist, IBM Plex Sans — NOT
  arbitrary uploads; security + legibility risk)

White-label theming explicitly **excludes**: layout, component
shapes, interaction patterns. Those are SKYCOT-owned and
non-negotiable.

**Theme resolution order:**
1. Tenant's stored preference (gateway-backed)
2. User's stored preference (localStorage)
3. System preference (`prefers-color-scheme`)
4. Default (`skycot-light`)

**Implementation surface:** a `ThemeProvider` component sets
`data-theme` and `class="dark"` on `<html>` based on resolution.
A `useTheme()` hook exposes change + persistence. SSR-safe:
the initial theme is computed server-side from a cookie to
avoid flash-of-wrong-theme.

**Embed support:** approval card surface must theme correctly
when rendered inside a customer's iframe. The theming layer
does not depend on top-level body classes; `data-theme` cascades
through the React tree only.

### D4 — Brand direction (SKYCOT default theme)

These are the SKYCOT default theme values. They are **swappable**
once the SKYCOT brand lands. Until then, this is the holding
pattern. None of these decisions block Track E.

| Token | Value | Why |
|---|---|---|
| Font (sans) | Inter | Most readable UI font today |
| Font (mono) | Geist Mono | Pairs cleanly with Inter |
| Type scale | 1.125× modular (12 / 14 / 15 / 16 / 18 / 20 / 24 / 30 / 36 / 48) | Restrained, Linear-density |
| Base size | 15px | Compromise between Linear-14 (compact) and 16 (comfortable) |
| Accent color | `oklch(0.55 0.22 264)` (indigo-600 equivalent) | Holds in light + dark; reads trustworthy; placeholder for SKYCOT brand |
| Neutral palette | slate (cool, slight blue tint) | Cleaner than zinc, less stark than gray |
| Radius (small) | 4px | Buttons, inputs, badges |
| Radius (default) | 6px | Cards, dropdowns |
| Radius (large) | 12px | Big cards (worker profile, approval) |
| Density (desktop) | 32px row height | Compact, Linear-school |
| Density (touch) | 44px min touch target | Apple HIG / WCAG |
| Motion (state change) | 150ms ease-out | Linear-fast |
| Motion (route change) | 250ms ease-in-out | Subtle but felt |
| Dark mode | First-class, system-driven | Operator persona reads logs at 2am |

OKLCH is used instead of HSL/RGB because Tailwind v4 supports it
natively and it produces perceptually-uniform color scales —
essential for synthesizing 11-stop palettes from a single hue
input (the per-tenant accent-color customization).

### D5 — Primitive components (Storybook coverage day 1)

Exactly 8 bespoke primitives. Everything else (modals, dropdowns,
tabs, accordions) is pulled from shadcn as-needed.

1. **Button** — variants: primary, secondary, ghost, destructive, link; states: default, loading, disabled; sizes: sm, md, lg, icon
2. **Card** — slots: header, body, footer; variants: default, hoverable, selected
3. **Avatar** — image + initials fallback; status ring (idle / working / waiting / blocked); sizes: xs, sm, md, lg
4. **StatusPill** — semantic colors (success / warning / error / info / muted); content: dot + label; sizes: sm, md
5. **ApprovalCard** — the load-bearing artifact from `docs/skycot/ux-research/approver.md`. Required slots: worker avatar + name, action description, consequence (if YES / if NO), three buttons max (approve / refuse / why), optional undo-countdown badge
6. **NavDrawer** — responsive: mobile bottom-sheet, tablet+ side-rail; shared API
7. **Toast** — variants: default, success, warning, error; optional action button (undo); auto-dismiss with progress bar
8. **DataCell** — variants: text, number, timestamp, status, action-menu; for tables (audit timeline, activity feed)

Each ships with: TypeScript types exported, Storybook story per
variant, accessible by default (keyboard navigation, screen reader
labels), themed via CSS variables, mobile + dark tested.

### D6 — Quality gates for primitives

Each primitive PR must include:
- Vitest unit test (props rendering, accessibility names)
- Storybook story per variant
- One Playwright e2e test from the smoke-page (covers all primitives in one go)
- Light + dark theme screenshots in PR body
- Bundle-size delta < 5KB gzipped per primitive

### D7 — File / LOC budgets

Match the Rust-side rule: **no component file > 500 LOC**. If a
component exceeds 500 LOC, split into sub-components.

## Consequences

**Positive:**
- Track E briefs parallelize cleanly; all reference this ADR for
  decisions instead of re-litigating.
- Per-tenant theming is built in from day 1, not retrofitted —
  retrofitting CSS-variables onto a hardcoded codebase is the
  single most expensive mistake in design-system work; we avoid
  it.
- SKYCOT brand can land at any time without rewriting components
  — only swap the default `:root` values.
- shadcn's "we own the code" model means no dependency drift; an
  upstream breaking change to a component library can never break
  us.

**Negative:**
- Initial setup investment (~2–3 days of focused work) before any
  user-visible surface ships. The build-then-validate preference
  amplifies this risk — we ship the scaffold before testing
  whether it produces a UX people like. Mitigation: the smoke
  path (D5 ApprovalCard against the existing gateway WebInbox)
  is the first slice; we get it in front of you within this
  same setup phase.
- shadcn copy-paste model means we maintain primitive code
  ourselves; no upstream patches arrive automatically. Mitigation:
  primitives are <500 LOC each; review burden is small.
- pnpm + Node 25 toolchain is a new dependency for the repo;
  contributors need Node installed. Mitigation: README + a
  `web/.tool-versions` file pin versions; CI does the same.

## Alternatives considered

- **Remix** — comparable to Next; rejected because Next's server-
  component model fits the audit-timeline use case better and the
  ecosystem (next-auth, examples) is larger.
- **SvelteKit** — smaller learning curve; rejected because the
  team will iterate against established React patterns and
  shadcn is React-only.
- **MUI / Ant Design** — large prebuilt component sets; rejected
  because the dependency-as-design-language trap is real (every
  customization fights the library). shadcn's copy-paste model
  avoids it.
- **CSS Modules + custom design system** — total control; rejected
  because the time-to-first-screen is 6× longer than
  shadcn-bootstrap.
- **One theme only, retrofit theming later** — fastest to ship;
  rejected because of D3 architecture point — retrofitting
  CSS-variables is the most expensive design-system mistake.

## Open follow-ups (not blocking this ADR)

- SKYCOT brand identity (logo, wordmark, full color system).
  Tracked in PRODUCT_ROADMAP.md "Open questions for the operator."
- Tenant-theming UI surface (how does an Operator configure their
  theme?). Captured in Track E backlog under the Operator
  console.
- Embedded approval card iframe contract — needs a separate brief
  once Track E begins implementing the approvals inbox.
- Storybook hosting (Chromatic? self-hosted? GitHub Pages?). Park
  until first preview pressure.

## How to use this ADR

- Any frontend brief queued to the swarm MUST link to this ADR
  and inherit its decisions.
- Any frontend PR that adds a hardcoded color, font size, or
  motion duration MUST be rejected; replace with a CSS variable.
- Any frontend PR that adds a `package.json` dependency without
  flagging it in the PR body MUST be rejected; bundle-size and
  audit-surface decisions are explicit, not implicit.
- If a brief proposes deviating from this ADR, the deviation
  goes through the operator BEFORE merge — same rule as
  CONTRACT-PR amendments.
