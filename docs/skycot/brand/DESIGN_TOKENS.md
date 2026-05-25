# SKYCOT design tokens

The token system is implemented as CSS custom properties in
[`web/app/globals.css`](../../../web/app/globals.css). This
document is authoritative for **token names and intended values**;
the CSS file is authoritative for **runtime behavior** (including
per-tenant overrides). When the two disagree, fix both — never
just one.

All tokens are themeable. The contract is:

- Every visual choice is a CSS custom property on `:root`.
- Per-theme overrides live on `[data-theme="..."]` selectors.
- Component code never hardcodes a color, radius, font size, or
  duration literal.

Internal brand-specimen pages may display locked token literals as
reference data; reusable product UI still consumes runtime tokens.

---

## Color

One primary brand gradient. Secondary palette colors exist for worker
avatars and semantic product moments, not for competing page chrome.

### Brand

| Token | Light | Dark | Use |
|---|---|---|---|
| `--brand-primary` | `#8B5CF6` SKYCOT purple | lifted purple for dark mode | Primary chrome, focus rings, active states, gradient start. |
| `--brand-primary-foreground` | `#FFFFFF` | near-black on lifted fills | Foreground color on `--brand-primary` fills. |
| `--brand-pink` / `--gradient-end` | `#EC4899` | lifted pink for dark mode | Gradient partner for brand marks, CTAs, and hero emphasis. |
| `--brand-coral` | `#FF6B6B` | lifted coral for dark mode | Third stop for high-energy hero emphasis gradients. |
| `--brand-teal` | `#14B8A6` | lifted teal for dark mode | Worker/cohort palette accent, not primary page chrome. |
| `--brand-ink` | `#1E1B4B` | light ink in dark mode | Warm deep-navy ink on light product surfaces. |
| `--brand-gradient` | `linear-gradient(135deg, #8B5CF6, #EC4899)` | theme-adjusted equivalent | Brand marks, primary CTAs, and canonical wordmark panels. |
| `--brand-gradient-3` | `linear-gradient(135deg, #8B5CF6, #EC4899, #FF6B6B)` | theme-adjusted equivalent | Hero emphasis text and deliberate high-energy moments. |

The 2026-05-19 D3 brand-consistency pass supersedes the older indigo
single-accent treatment. SKYCOT v1.0 uses the warm purple -> pink
gradient across brand-loud surfaces. The older purple -> cyan gradient
from the 2026-04 spec is not used for v1.0 page chrome.

`--primary` (the existing token in `globals.css`) is wired to
`--brand-primary` — they refer to the same color. The brand-* alias
exists so brand surfaces can address the brand explicitly while
generic components keep consuming `--primary`.

### Neutrals

Defined per-theme in `globals.css` as `--background`, `--foreground`,
`--card`, `--card-foreground`, `--popover`, `--popover-foreground`,
`--muted`, `--muted-foreground`, `--secondary`, `--secondary-foreground`,
`--accent`, `--accent-foreground`, `--border`, `--input`, `--ring`.

The light neutral surface is warm near-white (`#FAFAFF`) with
`#1E1B4B` foreground ink so product pages read warm rather than cold.

### Semantic

Reserve these for meaning, never decoration.

| Token | Meaning |
|---|---|
| `--destructive` / `--destructive-foreground` | Irreversible / dangerous actions, hard failures. |
| `--success` / `--success-foreground` | Completed runs, healthy state, approvals. |
| `--warning` / `--warning-foreground` | Soft failures, degraded state, quota close. |
| `--info` / `--info-foreground` | Neutral system messages, inline notes. |

If you need two states in a view, use these — not two competing page
accents. See `IDENTITY.md` § Forbidden visual patterns.

---

## Typography

Three loaded families. No more.

- **Sans (`--font-sans`)** — Inter. Loaded via `next/font/google`
  in `web/app/layout.tsx` with `--font-sans-loaded`. Weights 400,
  500, 600, 700.
- **Mono (`--font-mono`)** — JetBrains Mono. Same loader pattern.
  Weights 400, 500, 600.
- **Display (`--font-audiowide`)** — Audiowide. Loaded via
  `next/font/google` with `--font-display-audiowide`. Weight 400.

### Display family

Brand-loud display surfaces use `--font-audiowide`: wordmarks,
page-level hero statements, and deliberate editorial display moments.
The size changes by surface; the display voice does not. Inter remains
the body and dense-product family.

### Scale

8-step type scale. Sizes are absolute pixels; line-heights and
weights are scale-locked.

| Token | Size | Line height | Weight | Use |
|---|---|---|---|---|
| `--text-display-xl` | 72px | 1.0  | 400 | Marketing hero only; Audiowide. |
| `--text-display-lg` | 60px | 1.05 | 400 | Marketing section headers; Audiowide. |
| `--text-display-md` | 48px | 1.1  | 400 | Brand-loud in-product hero; Audiowide. |
| `--text-display-sm` | 36px | 1.15 | 400 | Compact page hero; Audiowide. |
| `--text-heading-xl` | 30px | 1.2  | 600 | Section headers in app pages. |
| `--text-heading-lg` | 24px | 1.25 | 600 | Subsection headers. |
| `--text-heading-md` | 20px | 1.3  | 600 | Card titles, modal headers. |
| `--text-heading-sm` | 18px | 1.35 | 600 | Compact group headers. |
| `--text-body-lg` | 16px | 1.5  | 400 | Default body. |
| `--text-body-md` | 14px | 1.5  | 400 | Dense UI body, table cells. |
| `--text-body-sm` | 12px | 1.4  | 400 | Captions, meta, footnotes. |
| `--text-mono-md` | 14px | 1.5  | 400 | Code, IDs, timestamps. |
| `--text-mono-sm` | 12px | 1.4  | 400 | Inline IDs, log lines. |

Tracking: display, body, and mono all use `letter-spacing: 0`.

**Do not** introduce sizes outside this scale. If you need a size
that isn't here, raise it as a follow-up brief that updates this
table and the matching CSS — don't just add a literal pixel value.

---

## Spacing

8pt grid. Every spacing decision (padding, margin, gap) snaps to a
multiple of 4px, with the 8px increment as the default beat.

| Token | Value | Common use |
|---|---|---|
| `--space-0` | 0 | Reset. |
| `--space-1` | 4px | Icon-to-text gap. |
| `--space-2` | 8px | Tight inline gap. |
| `--space-3` | 12px | Compact card padding. |
| `--space-4` | 16px | Default body gap. |
| `--space-5` | 24px | Section padding. |
| `--space-6` | 32px | Major group padding. |
| `--space-7` | 48px | Page-section gap. |
| `--space-8` | 64px | Hero-vertical gap. |
| `--space-9` | 96px | Marketing-section breathing room. |

Tailwind's default scale already matches the 8pt grid (`p-2`, `p-4`,
`p-6` etc.); these tokens exist for CSS authoring outside Tailwind
classes (CSS modules, inline custom properties). Prefer the Tailwind
utilities in component code.

---

## Radius

Three radii. Already defined in `globals.css`.

| Token | Value | Use |
|---|---|---|
| `--radius-sm` | 4px | Inputs, chips, inline badges. |
| `--radius-md` | 6px | Buttons, default surfaces. |
| `--radius-lg` | 12px | Cards, modals, popovers. |

No fully rounded (`9999px`) elements except: avatar circles, status
dots. Pill-shaped buttons are off-brand.

---

## Motion

Three durations. All `ease-in-out`. Defined in `globals.css` under
the `@theme` block; aliased as `--motion-fast`, `--motion-base`,
`--motion-slow` for component code that doesn't go through Tailwind's
`duration-*` utilities.

| Token | Value | Use |
|---|---|---|
| `--motion-fast` | 150ms | Hover, focus, press feedback. |
| `--motion-base` | 250ms | Most state transitions (toggle, expand, dismiss). |
| `--motion-slow` | 400ms | Deliberate moments — modal open, route change, drawer reveal. |

Easing is always `ease-in-out` (`cubic-bezier(0.4, 0, 0.2, 1)`).
Do not introduce per-component easing curves.

Motion serves comprehension. See `IDENTITY.md` § Motion.

---

## Iconography

- **Lucide React only** (`lucide-react`). Never mix icon libraries;
  never inline custom SVG icons in components.
- Stroke: 1.5px (the Lucide default; don't override).
- Color: `currentColor` (inherit from parent text color).
- Size: `size-4` (16px) in dense UI, `size-5` (20px) in headers,
  `size-6` (24px) in hero / empty-state imagery.

Custom brand glyphs (the wordmark, the favicon shape) live in
`web/public/brand/` and `web/public/favicon/` and are referenced
explicitly — they are not Lucide icons and don't follow the above
rules.

---

## Wordmark

- Canonical page-level wordmark: `SKYCOT` rendered in Audiowide 400,
  uppercase, `letter-spacing: 0`.
- Canonical wordmark panel: white Audiowide wordmark on
  `--brand-gradient` (`#8B5CF6` -> `#EC4899`).
- Legacy SVG assets under `web/public/brand/` remain available for
  small chrome slots that still need an image asset, but new brand
  references should render the D3 Audiowide lockup directly.
- Minimum legible size: 16px tall. Below that, use the favicon
  shape instead.

**Don't:**

- Place the wordmark over photography or a non-brand gradient.
- Recolor the wordmark away from white-on-gradient or brand-ink on
  light surfaces.
- Add a tagline lockup with the wordmark — the wordmark stands
  alone. Taglines live in adjacent type.
- Rotate, skew, distort, outline, drop-shadow, or 3D-extrude it.

---

## Favicon

- Vector: `web/public/favicon/favicon.svg` — a single brand-primary
  diamond, the simplest mark that reads at 16×16.
- Raster fallbacks: `web/public/favicon/icon-192.png` and
  `web/public/apple-touch-icon.png` (180×180).
- Next.js metadata routes: `web/app/icon.tsx` (renders the SVG
  diamond) and `web/app/apple-icon.tsx` (180×180 rounded variant).

The diamond shape is intentionally non-figurative — it doesn't
mean "sky" or "scout" literally. Brand marks earn meaning over
time; they don't carry it on day one.

---

## Token contract for downstream briefs

Every UI brief that lands after this one **must**:

- Consume existing tokens. No new color literals, no new font
  sizes, no new spacing primitives, no new motion durations.
- Extend the system via this document + `globals.css` in the same
  diff when a token genuinely doesn't exist. Token sprawl is
  forbidden, but token gaps should be filled, not worked around.
- Use Tailwind utilities (`bg-primary`, `text-foreground`, `p-4`,
  `duration-base`) in component code; reach for the raw
  `var(--token)` only in CSS modules or `style={}` props.

The token system is locked, not finished. Refinements ship via
scoped follow-up briefs.
