# Wave-6 Audit #3 — Dark-Mode + Multi-Tenant Theming Coverage

**Author:** Wave-6 audit agent #3 (read-only).
**Date:** 2026-05-21.
**Scope:** `web/app/**`, `web/components/**`, `web/lib/theme/**`,
`web/app/globals.css`.
**Method:** Static analysis only (server on :3001 returned HTTP 000
during audit; relied on grep + Read).
**Status of finding:** Documented; no code modified.

---

## TL;DR

Theming infrastructure is solid and **the toggle does flip the class
on `<html>`** — `ThemeProvider` wraps `NextThemesProvider`, `ThemePicker`
exposes Light/System/Dark + a tenant brand dropdown, and `globals.css`
defines a proper light/dark pair under both `:root` and `.dark`.

The problem is **adoption**: only the in-product chrome
(`/team`, `/cohorts`, `/workers`, `/audit`, `/orchestration`, `/hire`,
`/onboarding`, and most subcomponents inside `components/skycot/`)
actually consumes the global tokens. The **marketing surfaces**
(`/landing`, `/pricing`, `/compare`, `/customers`, `/changelog`, `/docs`,
`/for/<vertical>`) and the **auth shells** (`AuthEditorialShell`,
`WizardEditorialShell`) each redefine a parallel
`--ink / --bg / --line` palette **in hardcoded hex literals**, then
ignore `.dark` entirely. So a user who hits the dark toggle on `/`
gets a dark in-product surface — but the moment they click "Pricing"
or "Sign in", they land on a white-background page with hardcoded
`#1e1b4b` ink.

The Acme white-label demo proves the data-theme contract works for
the in-product chrome. It does not work for the marketing tree.

---

## 1. Theming infrastructure — what exists

### 1.a `ThemeProvider`
`web/components/skycot/ThemeProvider.tsx` wraps
`next-themes` (`attribute="class"`, `defaultTheme="system"`,
`enableSystem`) plus a `DataThemeBridge` that observes `.dark` on
`<html>` and writes `data-theme="<tenant>-light"` /
`-dark"` accordingly. Mounted from `web/app/layout.tsx:93`. Wraps
every route. **Working.**

### 1.b `ThemePicker`
`web/components/skycot/ThemePicker.tsx` exposes:
- Three mode buttons (Light / System / Dark) via `setTheme()` from
  `next-themes`.
- A `<select>` of tenant themes via `useTenantTheme().setThemeById`.
- Persists tenant choice to `localStorage["skycot.tenantTheme"]`.

Imported on `/` (`web/app/(app)/page.tsx:24`). **Working.**

### 1.c Theme registry
`web/lib/theme/themes.ts` ships two built-in themes:
- `SKYCOT_THEME` (`skycot-light` / `skycot-dark`)
- `ACME_THEME` (`acme-light` / `acme-dark`) — proves the tenant
  contract works end-to-end.

### 1.d Synthesis
`web/lib/theme/synthesize.ts` takes a single hex accent + neutral
tint + radius family and produces an entire OKLCH light+dark pair
on `[data-theme="<id>-light"]` / `[data-theme="<id>-dark"]`.
Designed for operator-console "configure theme" surface. **Working
contract; no callers yet — `synthesizeTenantCss` is referenced 0
times outside its own file.**

### 1.e Token surface in `globals.css`
Light tokens on `:root, [data-theme="skycot-light"]` and dark on
`.dark, [data-theme="skycot-dark"]`. Acme overrides at the bottom.
Token names cover background / foreground / card / popover / primary
/ secondary / muted / accent / destructive / success / warning /
info / border / input / ring + brand-primary + brand-gradient +
radius scale + motion + typography scale. **The contract document
(`docs/skycot/brand/DESIGN_TOKENS.md`) is correct; the CSS matches.**

---

## 2. The six dimensions, scored

### 2.1 Dark mode toggle exists + works — YES (partial)

`ThemePicker` is mounted on `/` and flips the `.dark` class via
next-themes. `MutationObserver` keeps `data-theme="<id>-<mode>"` in
sync. The in-product pages re-skin correctly. The marketing pages
do not.

The `ThemePicker` is **only** mounted on `/(app)/page.tsx` (the
approvals inbox). Other in-product pages — `/team`, `/audit`,
`/workers`, `/cohorts`, `/workflows`, `/orchestration`, `/skills`,
`/settings` — and ALL marketing / auth / onboarding pages have no
toggle. The user can flip dark mode on `/` but the moment they
navigate elsewhere, there is no UI to switch back.

**Verdict:** WORKS where it lives. UI for it is missing on 12+ pages.

### 2.2 CSS vars cover all colors — YES on paper, NO in practice

The token surface is complete. The problem is **bypass**:

| Metric | Count |
|---|---|
| Files using global `var(--...)` tokens | 224 (of 334 tsx files) |
| Files containing hardcoded `#8b5cf6` (brand purple) hex | 41 |
| Total occurrences of `#8b5cf6` (purple) | 171 |
| Total occurrences of `#ec4899` (pink) | 116 |
| Total occurrences of `var(--brand-primary)` | 20 |
| Total occurrences of `var(--brand-gradient)` | **0** |
| Total occurrences of `linear-gradient(` (hardcoded) | 122 |
| Total occurrences of `background: white;` | 69 (in 20 files) |

`--brand-gradient` is defined in `globals.css:117` but **referenced
zero times** anywhere else. Every gradient (the AppShell brand mark,
the BrandLoudHero hero, the auth shell logo, the landing-page logo,
the WorkerHero avatar fallback) hardcodes
`linear-gradient(135deg, #8b5cf6, #ec4899)`.

### 2.3 Audiowide hero on dark bg — INHERENTLY SAFE FOR THE HERO ITSELF, but BROKEN FOR THE PAGE

The `BrandLoudHero` (used on `/`, `/team`, `/cohorts`) has its
own purple→pink→coral gradient background and white text — it works
fine on top of a dark page background.

Outside the hero, the marketing-page hero text on `/landing` /
`/pricing` / `/customers` uses `var(--ink)` with `--ink: #1e1b4b`
locally defined. In dark mode the page does not get a dark
background (those pages don't respond to `.dark`), so the
`#1e1b4b` ink stays on `#fafaff` paper — no contrast problem,
but no dark mode at all.

If we made the marketing pages dark-aware, the Audiowide hero
gradient (purple→pink) over a `#0a0518` background would need a
brightness boost — the current `oklch(0.55)` chroma on purple is
borderline against very-dark navy. The dark-theme override in
`globals.css:169` raises `--brand-primary` to `oklch(0.72 0.16 256)`
already, but this is only consumed by 20 files (per dim 2.2),
not by the 122 hardcoded gradients.

### 2.4 Multi-tenant white-label — 3/10

The `Acme` data-theme override only redefines `--primary`,
`--background`, `--foreground`, `--card`, `--muted`, `--border`,
`--input`, `--ring`. Of those, the chrome surfaces that use
`var(--primary)` (Button, focus ring, in-product shells) will
re-skin. **The 122 hardcoded gradients and 171 hardcoded `#8b5cf6`
literals will not.** Switching to Acme will leave the hero, the
landing page, the auth shells, the pricing page, the BrandLoudHero
banner, the AppShell brand mark, the workflows builder, the skills
catalog, the worker personality card, and the install-prompt PWA
banner all rendering SKYCOT purple — not Acme orange.

`synthesizeTenantCss` is theoretically capable of generating a
matching `[data-theme="<id>-dark"]` block, but no part of the SPA
calls it. There is no plumbing from a tenant `tenant_id` →
synthesized CSS → injected `<style>` block. Track-E (operator
console / tenant config UI) has not landed this.

### 2.5 Dark-mode-only color tokens — DEFINED CLEANLY in globals.css

The dark tokens are NOT hardcoded under `[data-theme=dark]`-style
opt-in rules in component files. They live exclusively in
`globals.css` under `.dark, [data-theme="skycot-dark"]` (line 167)
and `[data-theme="acme-dark"]` (line 224). That's the right pattern.
**Architecture: clean.**

The problem is just adoption (dim 2.2).

### 2.6 Per-page dark coverage — 17 pages broken or partial

| Bucket | Count | Pages |
|---|---|---|
| BROKEN (hex bg + zero global token use, no dark response at all) | 8 | `/landing`, `/pricing`, `/customers`, `/customers/[slug]`, `/docs`, `/changelog`, `/offline`, `/(app)/settings/security` |
| PARTIAL (mix of hex + tokens, dark partially renders) | 8 | `/compare`, `/(app)/settings`, `/(app)/workflows`, `/(app)/skills`, `/(app)/workflows/templates`, `/(app)/workflows/templates/[templateId]`, `/(app)/workflows/[verticalId]/[name]`, `/(app)/workflows/[verticalId]/[name]/build` |
| OK (token-driven, dark renders correctly) | 59 | `/(app)/page.tsx` (approvals inbox), `/(app)/audit/*`, `/(app)/team`, `/(app)/cohorts`, `/(app)/workers/*`, `/(app)/orchestration`, `/(hire)/hire/*`, `/(setup)/onboarding/*`, `/auth/*` (page level — but see shells below) |

Not enumerated: the `/for/<vertical>` family (6 pages) all share
`VerticalLandingShell.tsx` which is itself BROKEN (16 hardcoded
purple hex literals, parallel `--ink/--bg/--line` palette, no
`.dark` response). Same for `AuthEditorialShell.tsx` and
`WizardEditorialShell.tsx`. These shells are reused across `/auth/signup`,
`/auth/verify`, `/auth/signin`, `/(setup)/onboarding/*` — so although
the page files themselves count as OK, the shell they render
inside is not dark-aware.

---

## 3. Top 5 hardcoded-light-color callsites blocking dark mode

Ranked by impact (number of surfaces affected × visibility):

### 3.1 `web/app/landing/page.tsx` — 21 hardcoded brand-hex literals
The marketing homepage. Defines its OWN local palette
(`--purple/--pink/--coral/--teal/--ink/--ink-2/--ink-3/--bg/--line`)
on `.land` with **hex literals** at lines 136–144. Sets
`background: var(--bg)` where `--bg = #fafaff`. No `.dark` or
`@media (prefers-color-scheme: dark)` selector exists anywhere in
the file. **Result:** dark toggle has zero effect on the entire
landing experience. 15 hardcoded gradients in this file alone.

### 3.2 `web/components/auth/AuthEditorialShell.tsx` — same local palette
Used on `/auth/signup`, `/auth/verify`, `/auth/signin`. Defines
`.auth-editorial { --ink: #1e1b4b; --bg: #fafaff; --line: #ece9f5; ... }`
at lines 46–57. No dark variant.
`web/components/auth/WizardEditorialShell.tsx` does the same for
the `/(setup)/onboarding/*` shell. Both are gateway components for
new-user onboarding — the first impression of the SPA. If the
visitor's OS is dark, they get a white flash.

### 3.3 `web/app/for/_components/VerticalLandingShell.tsx` — 16 purple hex literals
Used by all six `/for/sales`, `/for/support`, `/for/marketing`,
`/for/ops`, `/for/finance`, `/for/recruiting` pages. Same hardcoded
local palette + 9 hardcoded gradients + no `.dark` response.
**Result:** all six vertical landing pages broken.

### 3.4 `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — 21 hex literals
The workflow builder canvas. 2 token references, 21 hex literals
(`background: white`, `color: #71717a`, `border-color: #8b5cf6`,
`background: #fafaff`). The most prominent in-product page that
breaks in dark mode. The React Flow panel buttons explicitly set
`background: white` at line 561.

### 3.5 `web/components/skycot/AppShell.tsx` — gradient on brand mark
The shell wrapping every `/(app)/*` route. Uses tokens correctly
EXCEPT the brand-mark logo at line 87 which inline-styles
`background: "linear-gradient(135deg, #8b5cf6, #ec4899)"`. This
single literal means switching tenants (Acme orange) leaves the
brand mark purple — visible bleed of SKYCOT brand into a
white-labelled deployment.

---

## 4. Surface-by-surface dark-mode verdict

| Surface | Verdict | Notes |
|---|---|---|
| `/landing` (marketing home) | BROKEN | 21 hex, 15 gradients, local palette, no `.dark` |
| `/` (approvals inbox) | OK | Token-driven; ThemePicker mounted here |
| `/team` | OK | Token-driven (19 var refs, 0 hex bg) |
| `/cohorts` | OK | Token-driven |
| `/workers` + `/workers/[id]/overview` | OK | Token-driven |
| `/workers/[id]/skills/[skillId]/canvas` | OK | 18 var refs, 0 hex |
| `/workflows` | RENDERS WITH ISSUES | 14 brand purple hex + 4 `background: white` |
| `/workflows/[verticalId]/[name]/build` | BROKEN | The workflow builder — 21 hex literals, 2 tokens |
| `/audit` + `/audit/action/[id]` + `/audit/action/[id]/replay` | OK | Token-driven; all audit subcomponents clean |
| `/orchestration` | OK | Token-driven (6 var refs) |
| `/skills` | RENDERS WITH ISSUES | 7 hex literals (purple chip + white card bg + red error #991b1b) |
| `/skills/[skillId]` | RENDERS WITH ISSUES | 1 hex |
| `/settings`, `/settings/members`, `/settings/security`, `/settings/api-keys` | RENDERS WITH ISSUES → BROKEN | `security` has 31 hex literals, 0 tokens — fully broken; others partial |
| `/compare` | RENDERS WITH ISSUES | 6 hex + 5 gradients |
| `/pricing` | BROKEN | 8 hex + 9 gradients + local palette |
| `/customers`, `/customers/[slug]` | BROKEN | local palette, 11+13 brand hex |
| `/docs`, `/changelog` | BROKEN | local palette, no `.dark` |
| `/for/<vertical>` (all 6) | BROKEN | shared VerticalLandingShell broken |
| `/hire`, `/hire/step/[step]`, `/hire/done/[workerId]` | OK | HireShell uses tokens cleanly |
| `/(setup)/onboarding/*` (5 pages) | RENDERS WITH ISSUES | Page bodies token-driven; shell is broken |
| `/auth/signup`, `/auth/verify`, `/auth/signin` | RENDERS WITH ISSUES | Page bodies clean; shell is broken |
| `/offline`, `/maintenance`, `/global-error` | BROKEN | Static error states with hex |
| `/(app)/brand`, `/(app)/brand/components` | OK | The brand demo page — token-driven, ironically |

---

## 5. Multi-tenant theming readiness

**Overall score: 3/10.**

Working:
- Theme provider machinery (DataThemeBridge, MutationObserver, localStorage persistence).
- The token system in `globals.css` for the 224 files that consume it.
- `synthesizeTenantCss` can generate a full pair from one hex.
- `Acme` demo proves the data-theme contract works.

Blocked:
- 171 occurrences of `#8b5cf6` and 116 of `#ec4899` will not re-skin.
- `--brand-gradient` token is unused. Switching to Acme leaves all
  122 hardcoded gradients purple→pink.
- Marketing tree (8 pages + 6 vertical pages) and auth shell
  bypass the whole system.
- No server-side path to inject synthesized CSS into a tenant-scoped
  layout. `synthesizeTenantCss` has 0 callers.
- No CI lint rejects new `#8b5cf6` literals in component files
  (the brief-lint hook covers Rust crates, not `web/`).

---

## 6. Recommendation on dark mode in the demo

**Do not demo dark mode in the v1 launch demo unless the demo path
is strictly: `/landing → /` (sign in) → `/(app)/page.tsx` (approvals)
→ `/(app)/audit/...`.**

Within that path everything is token-driven and dark mode works
cleanly. If the demo deviates to `/workflows/[..]/build`,
`/settings/security`, `/customers`, `/pricing`, or any `/for/*`
vertical page, the user will see a white-background page in dark
mode and the illusion breaks.

The minimum-viable fix to unblock dark mode for the demo is:
1. Migrate `AuthEditorialShell` and `WizardEditorialShell` to global
   tokens (removes ~30 hex literals from the first-impression flow).
2. Migrate `AppShell.tsx` line 87 brand-mark gradient to
   `var(--brand-gradient)`.
3. Migrate `web/app/landing/page.tsx` to global tokens (single
   highest-impact file).

That alone covers the most-likely demo path. The 17 broken/partial
pages remaining are not on the v1 demo path and can ship as
"light-only for v1" with a `[data-theme=dark] { color-scheme: light; }`
override per-route as a safety hatch.

Longer-term remediation:
- Author a brief: `track-design-darkmode-marketing-pages-migrate`
  (replace the 6 marketing pages' local palette with global tokens).
- Author a brief: `track-design-brand-gradient-token-adopt` (replace
  122 hardcoded gradient literals with `var(--brand-gradient)`).
- Add a CI lint rule that rejects `#8b5cf6`, `#ec4899`,
  `background: white`, and `linear-gradient(135deg, #` in
  `web/app/**` and `web/components/**` outside `web/app/globals.css`.
- Wire `synthesizeTenantCss` into the operator console (Track-E).

---

## 7. Appendix — file lists

### Files defining a parallel local `--ink/--bg/--line` palette (bypass theme)
- `web/app/landing/page.tsx`
- `web/app/pricing/page.tsx`
- `web/app/docs/page.tsx`
- `web/app/changelog/page.tsx`
- `web/app/customers/page.tsx`
- `web/app/customers/[slug]/page.tsx`
- `web/app/for/_components/VerticalLandingShell.tsx`
- `web/components/auth/AuthEditorialShell.tsx`
- `web/components/auth/WizardEditorialShell.tsx`

### Files with `background: white` literal (20 files, 69 occurrences)
- `web/app/docs/page.tsx`
- `web/app/pricing/page.tsx`
- `web/app/for/_components/VerticalLandingShell.tsx`
- `web/app/compare/page.tsx`
- `web/components/pwa/InstallPrompt.tsx`
- `web/app/changelog/page.tsx`
- `web/components/skycot/CommandPalette.tsx`
- `web/components/skycot/worker/WorkerPersonalityCard.tsx`
- `web/app/(app)/workflows/page.tsx`
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`
- `web/app/customers/page.tsx`
- `web/app/customers/[slug]/page.tsx`
- `web/app/(app)/skills/page.tsx`
- `web/app/(app)/workflows/templates/page.tsx`
- `web/app/(app)/workflows/templates/[templateId]/page.tsx`
- `web/app/landing/page.tsx`
- `web/app/(app)/settings/page.tsx`
- `web/app/(app)/settings/security/page.tsx`
- `web/app/(app)/settings/api-keys/styles.tsx`
- `web/app/(app)/settings/members/page.tsx`

### Files using `var(--brand-primary)` (only 10 files, 20 total refs)
- `web/app/globals.css` (5)
- `web/components/ui/sonner.tsx` (2)
- `web/components/ui/TestimonialCard.tsx` (2)
- `web/components/ui/PricingCard.tsx` (2)
- `web/components/ui/Footer.tsx` (2)
- `web/components/ui/FeatureSection.tsx` (2)
- `web/app/(app)/brand/page.tsx` (2)
- `web/components/ui/WorkerCard.tsx` (1)
- `web/components/ui/LandingHero.tsx` (1)
- `web/app/(app)/brand/components/page.tsx` (1)

### Files referencing `var(--brand-gradient)`: **none**.

---

*End of audit.*
