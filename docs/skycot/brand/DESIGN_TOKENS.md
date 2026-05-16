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

---

## Color

One brand accent. One. Multiple accent colors signal a brand that
doesn't know what it stands for.

### Brand

| Token | Light | Dark | Use |
|---|---|---|---|
| `--brand-primary` | `oklch(0.55 0.18 256)` deep indigo | `oklch(0.72 0.16 256)` lighter indigo | Single brand accent. Wordmark, primary CTAs, focus rings, brand chrome. |
| `--brand-primary-foreground` | `oklch(0.99 0.005 256)` near-white | `oklch(0.14 0.020 256)` near-black | Foreground color on `--brand-primary` fills. |

Hue 256° (deep indigo) is the operator default. The decision was
indigo vs warm-coral; indigo wins for enterprise neutrality and
better dark-mode contrast. See `IDENTITY.md` for forbidden patterns
(no rainbow gradients, no secondary accent).

`--primary` (the existing token in `globals.css`) is wired to
`--brand-primary` — they refer to the same color. The brand-* alias
exists so brand surfaces can address the brand explicitly while
generic components keep consuming `--primary`.

### Neutrals

Defined per-theme in `globals.css` as `--background`, `--foreground`,
`--card`, `--card-foreground`, `--popover`, `--popover-foreground`,
`--muted`, `--muted-foreground`, `--secondary`, `--secondary-foreground`,
`--accent`, `--accent-foreground`, `--border`, `--input`, `--ring`.

The neutrals are cool-leaning (hue ≈ 248°–250°) to harmonize with
the indigo brand accent.

### Semantic

Reserve these for meaning, never decoration.

| Token | Meaning |
|---|---|
| `--destructive` / `--destructive-foreground` | Irreversible / dangerous actions, hard failures. |
| `--success` / `--success-foreground` | Completed runs, healthy state, approvals. |
| `--warning` / `--warning-foreground` | Soft failures, degraded state, quota close. |
| `--info` / `--info-foreground` | Neutral system messages, inline notes. |

If you need two states in a view, use these — not two brand
accents. See `IDENTITY.md` § Forbidden visual patterns.

---

## Typography

Two families. No more.

- **Sans (`--font-sans`)** — Inter. Loaded via `next/font/google`
  in `web/app/layout.tsx` with `--font-sans-loaded`. Weights 400,
  500, 600, 700.
- **Mono (`--font-mono`)** — JetBrains Mono. Same loader pattern.
  Weights 400, 500, 600.

### Display family

`--font-display` is an alias for `--font-sans`. Display surfaces
(hero headings, marketing-page H1s) use the same Inter family at
heavier weights and tighter tracking; we don't ship a separate
display face. Treat "Inter at 60px tracking -0.02em" as our display
type rather than introducing a second family.

### Scale

8-step type scale. Sizes are absolute pixels; line-heights and
weights are scale-locked.

| Token | Size | Line height | Weight | Use |
|---|---|---|---|---|
| `--text-display-xl` | 72px | 1.0  | 700 | Marketing hero only. |
| `--text-display-lg` | 60px | 1.05 | 700 | Marketing section headers. |
| `--text-display-md` | 48px | 1.1  | 700 | Landing-page H1 in-app. |
| `--text-display-sm` | 36px | 1.15 | 600 | Major page titles (`/team` empty state hero). |
| `--text-heading-xl` | 30px | 1.2  | 600 | Section headers in app pages. |
| `--text-heading-lg` | 24px | 1.25 | 600 | Subsection headers. |
| `--text-heading-md` | 20px | 1.3  | 600 | Card titles, modal headers. |
| `--text-heading-sm` | 18px | 1.35 | 600 | Compact group headers. |
| `--text-body-lg` | 16px | 1.5  | 400 | Default body. |
| `--text-body-md` | 14px | 1.5  | 400 | Dense UI body, table cells. |
| `--text-body-sm` | 12px | 1.4  | 400 | Captions, meta, footnotes. |
| `--text-mono-md` | 14px | 1.5  | 400 | Code, IDs, timestamps. |
| `--text-mono-sm` | 12px | 1.4  | 400 | Inline IDs, log lines. |

Tracking: display sizes (`-display-*`) use `letter-spacing: -0.02em`;
headings (`-heading-*`) use `-0.01em`; body uses `0`; mono uses `0`.

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

- File: `web/public/brand/skycot-wordmark.svg` (light variant —
  ink on transparent for use on `--background`).
- Dark variant: `web/public/brand/skycot-wordmark-dark.svg`
  (light-on-transparent for use on dark surfaces).
- Letterform: Inter Display weight 700, all lowercase "skycot",
  tracking 0.
- Color: fills with `currentColor` so the wordmark inherits from
  its parent — the brand surfaces use `--brand-primary`, the
  chrome surfaces use `--foreground`.
- Aspect ratio: ~3.6:1 (approx 360 × 100 in the SVG viewBox).
- Minimum legible size: 16px tall. Below that, use the favicon
  shape instead.

**Don't:**

- Place the wordmark over a photographic or gradient background.
- Recolor the wordmark to anything other than `--brand-primary`,
  `--foreground`, or pure `currentColor`-from-context.
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
