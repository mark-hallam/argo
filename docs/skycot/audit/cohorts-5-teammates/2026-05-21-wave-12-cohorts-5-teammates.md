# Wave-12 audit — /cohorts vs. 5-teammate hire reflection

**Date:** 2026-05-21
**Auditor:** Wave-12 audit agent #4
**Scope:** `/cohorts` page (web SPA) — department-tab coverage, brand-loud D3 register, phantom-worker carry-over from wave-4
**Refs:**
- Locked design: `docs/skycot/design-explorations/2026-05-18/cohorts-page/option-D3-magazine-tiles.html`
- Operator memory: `project_cohorts_page_design_locked.md` ("D3 magazine tiles — 6 dept tabs + asymmetric lead/stacked/compact layout") + `project_pm_cohorts_peta_patrick.md` ("cohorts page updated with Project & Ops as 7th dept tab")
- Wave-4 finding: `docs/skycot/audit/data-integrity/2026-05-20-wave-4-data-integrity.md` (CR-3: phantom `wkr_01J0MAYA`)

---

## Headline finding (one line)

**The implemented `/cohorts` page has ZERO department tabs.** It is a flat 3-column card grid keyed by `vertical`. The locked D3 design (7 dept tabs + magazine-asymmetric lead/stacked/compact tiles) is **not implemented**. None of the 5 newly-hired teammates' departments are surfaced as a top-level tab because the tab row does not exist in the production code.

---

## 1. Does `/cohorts` render? What dept tabs?

**Renders:** Yes. `web/app/(app)/cohorts/page.tsx` renders fine (uses `useCohorts()` → mock `SAMPLE_COHORT_SUMMARIES`).

**Dept tabs:** **NONE.** The page is:

```
<BrandLoudHero ... />               ← gradient banner (Audiowide title, KPIs)
<sticky subtitle bar>               ← "5 cohorts across the fleet"
<grid of CohortCard tiles>          ← 3 cols, flat, no grouping
<CohortDetailDrawer />              ← side panel on click
```

No `<Tabs>`, no `cat-tabs` div, no `[role="tablist"]`, no `Department` label anywhere. Grep confirms (`grep -rn "department\|tab" web/app/(app)/cohorts/page.tsx` returns nothing structural — only the placeholder copy "Cohorts group teammates by department, skill family, or trust grade." in the empty-state hero).

**Implication:** The page does not look like the locked D3 mockup at all. It looks more like the rejected D1 / D2 card-grid variants.

---

## 2-7. Per-department tab presence (against locked D3 spec)

Locked D3 expects **7 tabs**: Support · Finance & Ops · Sales · Project & Ops · People & HR · IT & Dev-ex · Legal.

| # | Dept (locked D3 label) | Tab present? | Cohort(s) for this dept in mock? | Worker visible (per audit prompt)? | Notes |
|---|---|---|---|---|---|
| 1 | Support | NO tab | `support::trusted` (6 members), `support::developing` (4 members) | Jamal — appears inside `SAMPLE_COHORT_DETAIL.members` as `wkr_01J0JAMAL` (trust 8) | Cohort exists, but is rendered as a generic card "support · Trusted", not under a tab |
| 2 | Sales | NO tab | `sales::established` (3 members) | Priya — **NO**, Priya's vertical is `recruiting`, not `sales`. Priya is in the `support::trusted` cohort members list as `wkr_01J0PRIYA` (data-integrity mismatch) | Sales cohort has no actual member roster in `SAMPLE_COHORT_DETAILS` (only `support::trusted` is hydrated) |
| 3 | Marketplace / gtm-engineer | NO tab | `marketplace::developing` (2 members) | Mariah — **NO**, Mariah is not in any cohort members list (she is in `sample-team.ts` with `vertical: "marketplace"`, but `marketplace::developing` has no hydrated detail) | The wave-4 audit also called this out — Mariah missing from cohort data despite being THE marketplace teammate |
| 4 | Finance / Finance & Ops | NO tab | **No finance cohort in mock** | Sam — **NO**, Sam's vertical is `ops` not `finance`. He is in `support::trusted` members list as `wkr_01J0SAM` (data-integrity mismatch) | The audit prompt's mental model says "Finance tab — Sam" but Sam's seed says `ops`. No finance cohort exists in `SAMPLE_COHORT_SUMMARIES` at all. |
| 5 | Marketing | NO tab | `marketing::unknown` (1 member) | Nori — **NO**, Nori is not in the marketing cohort's member list; she is in `support::trusted` as `wkr_01J0NORI` (data-integrity mismatch) | Marketing cohort exists at summary level but has no detail-data hydration |
| 6 | Project & Ops | NO tab | **No Project & Ops cohort in mock** | Peta — **NO seed entry**; Patrick — **NO seed entry**. Per `project_pm_cohorts_peta_patrick.md` they should exist; per wave-4 (`Missing entities`) the operator memory has been written but seed data has NOT been added. | The "7th dept tab" the operator memo refers to **does not exist** in any form: no tab UI, no cohort summary, no worker seed. |

---

## 8. Phantom worker `wkr_01J0MAYA` — wave-4 carry-over status

**STILL PRESENT.** Single line:

```
web/lib/sample-cohorts.ts:123:      worker_id: "wkr_01J0MAYA",
```

`grep -r "wkr_01J0MAYA" web/` returns this exact one hit. There is **no** Maya entry in:
- `sample-team.ts`
- `sample-workers-detail.ts`
- any of the `sample-worker-*` files

This is the **same finding wave-4 (data-integrity, CR-3 SEVERE) flagged on 2026-05-20**, untouched. If the operator clicks into the `support::trusted` cohort drawer, the top-listed member (highest trust grade, 9) is a phantom. Any click-through to `/workers/wkr_01J0MAYA` will 404.

---

## D3 brand-loud register compliance

| D3 spec element | Implemented? | Where |
|---|---|---|
| Audiowide hero title | YES | `BrandLoudHero` uses Audiowide on the page-h h1 |
| Purple→pink→red gradient banner | YES | `BrandLoudHero` shared component |
| Page hero KPI strip (Cohorts / Members / Skills) | YES | `CohortsHero` passes 3 KPIs |
| 7 dept tabs (Support / Finance & Ops / Sales / Project & Ops / People & HR / IT & Dev-ex / Legal) | **NO** | not implemented |
| Asymmetric "magazine" grid (1 lead + 2 stacked + 4 compact) | **NO** | plain `grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4` of identical-sized cards |
| Per-cohort featured / new / trending badges | **NO** | only a trust-grade-band pill |
| Department section header ("Project & Ops · 2 just-launched cohorts" / underline rule + meta line) | **NO** | flat list, no section dividers |
| Per-tile descriptive paragraph (40-60-word feature copy) | **NO** | cards show member/skill counts only |
| Mono-font kicker + 5-star rating + "Hire your first Peta →" CTA inside each tile | **NO** | no CTA, no rating, no kicker |
| Lead-tile gradient blob background + 32px padding | **NO** | all tiles identical |

**Verdict:** the page has the hero **banner** in brand-loud register, but the **body** (the actual cohort browsing experience) is the un-locked D1/D2 plain-card variant. The page is approximately **20% D3-locked, 80% un-implemented** versus the locked design.

---

## Data-vs-design alignment table (5 teammates + 2 PMs)

| Teammate | Sample-team vertical | Cohort summary that *should* host them | Cohort detail member list | Dept tab in locked D3 | Implemented dept tab? |
|---|---|---|---|---|---|
| Mariah | marketplace | `marketplace::developing` | not hydrated (no detail entry) | (no marketplace tab in D3; closest is "Finance & Ops" or new "Marketplace") | NO tab |
| Priya | **recruiting** (not sales) | (none — no recruiting cohort) | mis-filed in `support::trusted` | (closest "People & HR") | NO tab |
| Jamal | support | `support::trusted` / `support::developing` | `support::trusted` (CORRECT) | Support | NO tab |
| Nori | marketing | `marketing::unknown` | mis-filed in `support::trusted` | (no marketing tab in D3) | NO tab |
| Sam | **ops** (not finance) | (none — no ops/finance cohort) | mis-filed in `support::trusted` | Finance & Ops | NO tab |
| Peta | — (no seed) | (none) | (none) | Project & Ops (per memory) | NO tab |
| Patrick | — (no seed) | (none) | (none) | Project & Ops (per memory) | NO tab |

**Data-integrity sub-finding:** the only hydrated cohort detail (`support::trusted`) lists **6 members from 4 different verticals** — Maya (phantom), Priya (recruiting), Jamal (support), Rio (real-estate), Nori (marketing), Sam (ops). That violates the cohort definition (`A cohort is a group of workers with the same vertical and a coarse trust_grade_band` — sample-cohorts.ts top comment). The mock fixture itself contradicts the contract the code documents.

---

## Top 3 dept-tab gaps (ranked by demo impact)

### Gap 1 — There are no tabs at all (DEMO-BLOCKER for "browse by department" narrative)

Severity: P0 for demo, P1 for ship.
The page hero subtitle says "Cohorts group teammates by department, skill family, or trust grade" but the body offers **no UI to browse by department**. Anyone clicking /cohorts during the demo sees a flat 5-card grid. Whatever narrative the operator wants to tell ("look — here's our Sales cohort, here's Project & Ops with Peta and Patrick") collapses because the visual frame is wrong.

### Gap 2 — Project & Ops tab does not exist anywhere in the stack (BLOCKS the PM-cohort launch story)

Severity: P1.
Per `project_pm_cohorts_peta_patrick.md` (2026-05-19) the "Project & Ops" cohort is the FIRST non-vertical cohort family, and the cohorts page was supposedly "updated with Project & Ops as 7th dept tab". Reality:
- No `Project & Ops` string in `web/app/`, `web/components/`, or `web/lib/`
- No `peta` / `patrick` worker seed entry
- No `pm` / `project-ops` cohort summary
- No corresponding department row in `sample-workers-detail.ts`

The operator memory has been written **as if shipped** but the work has not landed. This is the highest-priority drift between memory and code.

### Gap 3 — Phantom worker `wkr_01J0MAYA` carries over (wave-4 CR-3 unaddressed)

Severity: P1 for demo (was tagged SEVERE in wave-4 a day ago).
Same one-line phantom in `sample-cohorts.ts:123`. The only hydrated cohort drawer (the cohort the operator is most likely to demo because it's the only one with real members) opens with a top member that does not exist anywhere else in the system. Any click-through 404s. Wave-4 audit on 2026-05-20 flagged this. Today, 2026-05-21, the line is unchanged.

---

## Cohorts page implementation reality (file refs)

- Page entry: `web/app/(app)/cohorts/page.tsx` — 149 LOC, no tabs
- Card: `web/components/skycot/CohortCard.tsx` — generic card with trust-band pill
- Hook: `web/lib/hooks/use-cohorts.ts` → `web/lib/api/cohorts.ts`
- Mock data: `web/lib/sample-cohorts.ts` (5 summaries, 1 detail hydrated)
- Hero (shared): `web/components/skycot/BrandLoudHero.tsx` (Audiowide + gradient — this part IS D3-correct)
- Locked design: `docs/skycot/design-explorations/2026-05-18/cohorts-page/option-D3-magazine-tiles.html`

## Recommended remediation order

1. Add Peta + Patrick seed entries to `sample-team.ts` + `sample-workers-detail.ts` (no-code-change risk, unblocks demo narrative and aligns memory with reality).
2. Fix phantom Maya — replace `wkr_01J0MAYA` line in `sample-cohorts.ts:123` with `wkr_01J0MARIAH` (or remove the row + fix `member_count`). Closes wave-4 CR-3.
3. Re-hydrate `SAMPLE_COHORT_DETAILS` so each summary has a detail entry whose `members` actually share the cohort's `vertical` (today only `support::trusted` is hydrated, and its members come from 4 verticals).
4. Add a `pm::developing` (or `project-ops::developing`) cohort summary + detail containing Peta + Patrick.
5. Implement the D3 dept-tab strip (the largest piece of work; gates the locked-design claim).
6. Implement the asymmetric magazine grid (1 lead + 2 stacked + 4 compact).

Items 1-4 are data-layer; items 5-6 are UI. The data-layer fixes unblock the demo even if 5-6 ship later.
