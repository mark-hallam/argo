# Design exploration session — resume guide

**Last updated:** 2026-05-19
**Status:** All 10 page designs locked. Brand-consistency pass complete.

## What this is

The operator (Mark Hallam) and Claude went through a multi-day design-exploration session producing locked HTML mockups for every SKYCOT product surface using the **optometrist method** (3-5 wildly varied options per page, narrow, lock). All locked designs live under `docs/skycot/design-explorations/2026-05-18/`.

## What's locked

10 page designs (15 actual HTML files counting sub-pages):

| Surface | Locked file | Notes |
|---|---|---|
| Marketing landing | `marketing-landing/option-D3-with-audiowide.html` | The brand-language **reference**. Don't change. |
| Audit page | `audit-page/option-B3-enterprise.html` | Etherscan-style block list + pop-out modal |
| Workers | `workers-page/option-C2-holographic-showcase.html` | Trading-card deck with gradient banners |
| Approvals | `approvals-inbox/option-E-ops-dashboard.html` | KPI strip + severity-tier sections |
| Onboarding wizard (5 steps) | `onboarding-wizard/option-F1-editorial-center.html` (step 1) + `step-2-workspace.html` + `step-3-connect-provider.html` + `step-4-pick-vertical.html` + `step-5-first-hire.html` | Editorial centered single column |
| Cohorts | `cohorts-page/option-D3-magazine-tiles.html` | 6 dept tabs + asymmetric magazine layout |
| Worker detail | `worker-detail/option-C2-personality.html` | LinkedIn profile + trait chips + behaviors |
| Skills | `skills-page/option-A1-prominent-search.html` | Marketplace catalog + hero search bar |
| Settings | `settings-page/option-D-top-tabs.html` | Horizontal top tabs + sub-tabs |
| Auth (3 pages) | `auth-pages/option-A-editorial.html` (signup) + `page-verify.html` + `page-signin.html` | Editorial-center matching onboarding F1 |

Each surface has a corresponding `project_<page>_page_design_locked.md` memory entry documenting layout + components. **Trust those for structure**, but for **typography + color + background variables**, trust `project_brand_consistency_pass.md` (the 2026-05-19 brand pass).

## What the brand recipe is

After the 2026-05-19 brand-consistency pass, every page uses:

1. **Audiowide hero typography** — `font-family: 'Audiowide', 'Inter', sans-serif; font-weight: 400; letter-spacing: 0;` for the main `h1`. Size scales per surface (96px onboarding welcome down to 36px audit/approvals/settings).
2. **Audiowide SKYCOT wordmark** in every sidebar/topbar.
3. **3-color gradient text fill** (`--gradient-3: linear-gradient(135deg, purple, pink, coral)`) on `<strong>` emphasis inside `h1`.
4. **Body background**: `var(--bg)` = `#fafaff` + two fixed radial blobs via `body::before` (purple, top-right) and `body::after` (pink, bottom-left).
5. **CSS variables** — `--ink: #1e1b4b`, `--line: #ece9f5`, `--gradient: linear-gradient(135deg, purple, pink)` (note: **purple→pink**, NOT purple→cyan — overrides the official brand-spec gradient because the marketing landing diverged and the operator chose to keep that).
6. **Inter remains for body**, JetBrains Mono remains for technical data (hashes, mono pills, timestamps, keyboard hints).

## What's pending

- **Implementation briefs** — none authored yet. When ready, write one per surface at `docs/build-plan/streams/mission-<page>-rebuild.md` (e.g. `mission-audit-page-b3-enterprise-rebuild.md`). Each brief tells the swarm to rebuild the page in `marketing/` (Next.js, for the marketing landing) or `web/` (Next.js, for in-product surfaces) using the locked HTML as the spec.
- **Demo recording** — operator wants to record `docs/skycot/demos/v1-mission-mvp.mp4` walking through the customer journey. The 16-screen `open` command sequence is at the bottom of this doc.
- **MAYA character + illustrations** — only present on the marketing landing. Could be extended to other pages later if the operator asks for a "bigger brand pass."

## Stale items to know about

When reading the individual `project_<page>_page_design_locked.md` files, the following claims are STALE (overridden by the brand-consistency pass):

- "Inter throughout (no Audiowide on this page)" — false; every page now has Audiowide on its h1 + wordmark
- "Brand-restrained register" vs "brand-loud register" — the distinction collapsed when Audiowide was applied everywhere; what's now scaled is the **size** of the Audiowide hero, not its presence
- "Gradient `#8B5CF6 → #0891B2` (purple→cyan)" — superseded by `purple → pink`
- "`--ink: #18181B`" — superseded by `#1e1b4b` (purple-tinted)

The `project_brand_consistency_pass.md` memory is the authoritative typography/color spec.

## Demo flow (customer journey order)

```bash
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/marketing-landing/option-D3-with-audiowide.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/auth-pages/option-A-editorial.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/auth-pages/page-verify.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/onboarding-wizard/option-F1-editorial-center.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/onboarding-wizard/step-2-workspace.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/onboarding-wizard/step-3-connect-provider.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/onboarding-wizard/step-4-pick-vertical.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/onboarding-wizard/step-5-first-hire.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/workers-page/option-C2-holographic-showcase.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/worker-detail/option-C2-personality.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/approvals-inbox/option-E-ops-dashboard.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/audit-page/option-B3-enterprise.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/skills-page/option-A1-prominent-search.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/cohorts-page/option-D3-magazine-tiles.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/settings-page/option-D-top-tabs.html
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/auth-pages/page-signin.html
```

## To resume

If you're a Claude session opened in this repo and the operator asks about the design work:

1. **Don't rebuild.** Everything is locked.
2. Read this file + `project_brand_consistency_pass.md` to know the current state.
3. If the operator asks for **implementation briefs**, write them at `docs/build-plan/streams/mission-<page>-rebuild.md` referencing the locked HTML as the spec.
4. If the operator asks for **a new page** not in the list above, run the optometrist method per `user_design_exploration_optometrist_method.md`.
5. If the operator asks for **tweaks to a locked page**, edit the locked HTML directly + update its memory entry.
6. **Don't push from this session.** Per `feedback_no_push_handoff_to_conductor.md`, the conductor process owns commits/pushes. Write a handoff prompt instead.
