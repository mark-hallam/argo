# READ BEFORE ANY UI/DESIGN WORK — THE BRAND IS LOCKED

> **The SKYCOT brand and per-page design directions are already locked.**
> They were chosen via the operator's optometrist method (extreme poles →
> narrow → pick) **months ago** and shipped. Do **not** run a from-scratch
> design round on a page that already has a locked direction below. Consume
> the tokens and the locked direction. (See **THE LOAD-BEARING RULE** at the
> bottom — it is the whole point of this file.)

---

## The brand files (source of truth)

| File | What it locks |
|------|---------------|
| [`README.md`](./README.md) | How to use the brand docs when authoring a UI brief |
| [`IDENTITY.md`](./IDENTITY.md) | Positioning, voice & tone, forbidden patterns — *how SKYCOT sounds/feels* |
| [`DESIGN_TOKENS.md`](./DESIGN_TOKENS.md) | Color, typography, spacing, motion, iconography — *how SKYCOT looks*. Authoritative for token **names + values** |
| [`SKYCOT_BRAND_GUIDELINES.md`](./SKYCOT_BRAND_GUIDELINES.md) | The full 2026-04 brand style guide |
| `web/app/globals.css` | **Runtime token source** — the CSS custom properties (`--brand-*`, `--primary`, `--gradient-end`, …) are authoritative for *runtime behavior*. Consume these; never hard-code hexes that duplicate them |

## Locked brand fundamentals

- **Gradient = purple → PINK: `#8B5CF6 → #EC4899`** (`--brand-gradient` in `globals.css`).
  **Purple → cyan (`#8B5CF6 → #0891B2`) is DEPRECATED. Never use it.** It
  shipped in an early brand-guidelines draft and keeps creeping back into
  fresh design rounds — it is wrong. The 3-stop variant is purple → pink →
  coral (`#8B5CF6 → #EC4899 → #FF6B6B`, `--brand-gradient-3`).
- **Fonts**: **Audiowide** (display / hero headlines / wordmark), **Inter**
  (body + UI), **JetBrains Mono** (code).
- **Primary chrome**: solid SKYCOT purple `#8B5CF6` for buttons, focus rings,
  active states.

## The locked landing page

- **Locked design**: `docs/skycot/design-explorations/2026-05-18/marketing-landing/option-D3-with-audiowide.html`
- **Shipped at**: `web/app/landing/page.tsx` (+ `web/app/landing/page.css`)
- Register: **brand-loud-marketing**. Hero: *"Don't buy another tool. Hire a
  teammate. (Seriously.)"* It is **done and shipped** — do not re-explore it.

## Per-page locked-design map

Every row below was locked via the `docs/skycot/design-explorations/2026-05-18/`
optometrist round (or a later dated round) and recorded in an operator
`project_*_design_locked` auto-memory. **These pages are NOT open for a
from-scratch round.** Consume the named direction.

| Surface | Locked direction | Where the mockup lives |
|---------|------------------|------------------------|
| Landing | D3-with-audiowide (brand-loud-marketing) | `design-explorations/2026-05-18/marketing-landing/option-D3-with-audiowide.html` |
| Audit | B3-enterprise (Etherscan-style, Inter, pop-out modal; brand-restrained) | `design-explorations/2026-05-18/audit-page/` |
| Workers | C2-holographic-showcase (trading-card deck, brand-loud/viral) | `design-explorations/2026-05-18/workers-page/` |
| Approvals | E-ops-dashboard (KPI strip + severity tiers + SLA + donuts; brand-restrained) | `design-explorations/2026-05-18/approvals-inbox/` |
| Onboarding | F1 editorial-center across 5 steps (welcome/workspace/provider/vertical/hire) | `design-explorations/2026-05-18/onboarding-wizard/` |
| Cohorts | D3 magazine tiles (6 dept tabs, asymmetric lead/stacked/compact; brand-loud-editorial) | `design-explorations/2026-05-18/cohorts-page/` |
| Worker detail | C2 personality (LinkedIn-profile + trait chips + great-at/escalate-when; brand-loud-warm) | `design-explorations/2026-05-18/worker-detail/` |
| Skills | A1 marketplace + hero-search bar with suggested-search chips (brand-loud-marketplace) | `design-explorations/2026-05-18/skills-page/` |
| Settings | D top-tabs (horizontal tabs + sub-tabs + 2-col content; brand-restrained) | `design-explorations/2026-05-18/settings-page/` |
| Auth (signup/verify/signin) | A editorial-center (matches Onboarding F1; magic-link, no passwords) | `design-explorations/2026-05-18/auth-pages/` |
| Workflow goldens | sidebar + cohorts-D3 lead-tile pass on cool-paper bg | `design-explorations/2026-05-21/workflow-goldens-capture/option-F-hybrid-bg1-cool-paper.html` |
| Pitch (Portia) | brand-loud headline surface; see Track P briefs + DEC-008/DEC-016 | `docs/build-plan/streams/` (P-01..P-08) |
| GTM (Grant) | `/gtm` console; see DEC-018 | `docs/build-plan/streams/` |
| Genesis | AI-teammate onboarding studio; see DEC-010/DEC-025 | `docs/skycot/design/2026-05-30-genesis-done-right.md` |

> **D3 brand-consistency recipe (overrides any "Inter only / no Audiowide"
> claim in an individual design-lock memory):** Audiowide hero typography +
> purple→pink gradient (NOT purple→cyan) + warm radial-blob background +
> `#1e1b4b` ink, applied uniformly across the in-product surfaces. When a
> per-page memory and this recipe disagree, this recipe wins.

## The in-app brand reference page

- Lives at `web/app/_dev/brand/page.tsx` (+ `web/app/_dev/brand/components/`):
  live color swatches, typography samples, wordmark panel, component showcase.
- `_dev/` is a **Next.js private segment** (underscore-prefixed folders are
  exempt from routing — the architecture intro at the top of this repo's
  `CLAUDE.md` calls it Next.js App Router; in plain terms it means the folder
  is hidden so the page returns **404** to any logged-in user). It is an
  internal designer reference, not a product feature.
- **To view it:** open the files in your editor, or temporarily rename
  `web/app/_dev/` → `web/app/(app)/` **on a throwaway branch** (never commit
  that rename).

---

## THE LOAD-BEARING RULE

**The optometrist / from-scratch design method does NOT apply to
already-locked pages.** For any surface in the map above: **consume the tokens
+ the locked direction.** Do not produce wildly-varied HTML mockups for it, do
not re-pick a gradient, do not "explore" it again.

**Only run a NEW exploration for a genuinely new surface — and check this map
FIRST** to be sure it is actually new. When in doubt whether a page is locked,
assume it is and ask the operator before exploring.

This file exists because a fresh session once ran a full from-scratch
optometrist landing round when the brand, an example page, and the live
landing page were all built and locked long ago — and it reintroduced the
deprecated purple→cyan gradient. See
[`docs/skycot/decisions/DEC-027-brand-and-design-system-locked.md`](../decisions/DEC-027-brand-and-design-system-locked.md).
