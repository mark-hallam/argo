---
id: DEC-027
date: 2026-05-31
status: active
supersedes: none
---

# DEC-027 — Brand + design system are LOCKED; no optometrist round on locked pages

## Decision

The SKYCOT **brand** (gradient = purple→PINK `#8B5CF6 → #EC4899`; purple→cyan
DEPRECATED; fonts Audiowide / Inter / JetBrains Mono) and the **per-page
design directions** chosen via the operator's optometrist method are **locked**
and **shipped**. The single loud entry-point that captures this is
[`docs/skycot/brand/DESIGN_SYSTEM.md`](../brand/DESIGN_SYSTEM.md) — titled
*"READ BEFORE ANY UI/DESIGN WORK — THE BRAND IS LOCKED."*

**The load-bearing rule:** the optometrist / from-scratch "show me 3–5 wildly
varied mockups" method does **NOT** apply to a page that already has a locked
direction. For locked surfaces, **consume the tokens + the locked direction**.
Run a NEW exploration **only** for a genuinely new surface — and check the
per-page locked-design map in `DESIGN_SYSTEM.md` first to confirm it is new.

## Why

A fresh session ran a full from-scratch optometrist landing-page round when the
brand, an example brand-reference page (`web/app/_dev/brand/`), and the real
shipped landing page (`web/app/landing/page.tsx`, from the locked
`option-D3-with-audiowide.html` mockup) were **all built and locked long ago**.
Worse, the new round **reintroduced the deprecated purple→cyan gradient**
(`linear-gradient(135deg,#8B5CF6,#0891B2)`).

**Root cause:** none of the three docs a fresh session reads first
(`CLAUDE.md`, `docs/build-plan/RESUME.md`, the decision register) carried a
loud "the brand is LOCKED — read this first" pointer. The locked state lived
only in scattered `project_*_design_locked` auto-memories that a fresh session
doesn't load. This decision fixes that by making the lock impossible to miss.

## How (operative state)

- **Authored** `docs/skycot/brand/DESIGN_SYSTEM.md` — one screen: brand files,
  locked gradient/fonts, the locked landing design, the per-page locked-design
  map, the `_dev/brand` reference-page note, and THE LOAD-BEARING RULE.
- **Referenced loudly from the 3 fresh-session entry points:**
  - `CLAUDE.md` — `## Brand & design system (LOCKED)` section near the top.
  - `docs/build-plan/RESUME.md` — added to its "read these files" list.
  - This ADR + the index line in `docs/skycot/decisions/README.md`.
- **Added** `web/app/landing/README.md` — "this page is LOCKED, don't re-explore."
- **Deleted** the redundant `docs/skycot/design-explorations/2026-05-31/landing/option-*.html`
  round (landing is locked + shipped; that round reintroduced the deprecated
  cyan gradient).

The brand files themselves: `docs/skycot/brand/{README,IDENTITY,DESIGN_TOKENS,SKYCOT_BRAND_GUIDELINES}.md`
+ `web/app/globals.css` (runtime token source).

Related: [[DEC-003]] (Melbourne not Sydney — brand/marketing copy).
The optometrist method itself stays valid for **new** surfaces — this decision
only scopes it *out* of already-locked ones.
