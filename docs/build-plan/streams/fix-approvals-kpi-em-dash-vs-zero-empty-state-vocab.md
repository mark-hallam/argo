# fix-approvals-kpi-em-dash-vs-zero-empty-state-vocab

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-kpi-em-dash-vs-zero-empty-state-vocab`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-6-claim-6-trust-and-approvals.md` F11 [P3].

🟢 **P3 — Approvals page KPI strip shows em-dash ("—") for empty metrics: `Resolved/wk: —`, `Avg response: —`, `$ at risk: —`, `Approval rate: —`. But `Pending: 0` IS rendered as `0`. Inconsistent empty-state vocabulary on the same KPI strip — the em-dash reads "we have no data" while the 0 reads "we have data and it's zero." Pick one.**

## Root cause

The approvals page renders the locked E-ops-dashboard with a KPI (Key Performance Indicator) strip. Per memory `project_approvals_page_design_locked` (E-ops-dashboard, locked 2026-05-19), the dashboard uses 5 KPIs + 3 severity tiers + $ at risk + confidence donuts + 7-day approval rate ring.

The current rendering inconsistency:
- `Pending: 0` — numeric 0
- `Resolved/wk: —` — em-dash
- `Avg response: —` — em-dash
- `$ at risk: —` — em-dash
- `Approval rate: —` — em-dash

Em-dash is conventionally "data unavailable" (e.g. avg over 0 events is genuinely undefined). Zero is "data exists and is zero." Mixing these vocabularies on the same strip reads as a polish gap.

Normalization options:
- **(a)** All-zero: `Pending 0 / Resolved/wk 0 / Avg response 0s / $ at risk $0 / Approval rate 0%` — cleanest "everything is fine" zero state.
- **(b)** All-dash: every empty metric renders as `—`. Reads as "no data yet."
- **(c)** Hybrid: keep `Avg response: —` (genuinely undefined over 0 events) but switch the rest to numeric 0.

Per ops-dashboard convention, (c) is the most honest — averages over 0 events are mathematically undefined, but counts and rates over 0 events ARE 0. The audit recommendation also suggests (c).

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F11 [P3] — Approvals page KPI strip shows em-dash for empty metrics; reads slightly defensive vs. clean zero state

## Fix recipe

1. **Locate the KPI strip** at `web/components/skycot/approvals/KpiStrip.tsx` (or equivalent — `rg 'Resolved/wk' web/`).
2. **Apply option (c) normalization**:
   - `Pending`: stay as `0`
   - `Resolved/wk`: change `—` → `0`
   - `Approval rate`: change `—` → `0%`
   - `$ at risk`: change `—` → `$0`
   - `Avg response`: stay as `—` (genuinely undefined over 0 events) BUT add a small tooltip / aria-description "no resolved approvals yet"
3. **Convert literal em-dashes to a `formatKpi(value, fallback)` helper** so future KPIs use the same rule consistently. `formatKpi(value, "$0")` returns `"$0"` when value is 0; `formatKpi(undefined, "—")` returns the dash.
4. **Vitest pin** asserting the rendered KPI values in both empty (counts=0, denominator=0) and populated states.
5. **Verify with E-ops-dashboard design lock** — the rendering must still match the locked visual style (Inter font, severity tier colours, donut shapes etc.). No visual register change.

## Acceptance criteria

- [ ] `Resolved/wk`, `Approval rate`, `$ at risk` render `0` / `0%` / `$0` instead of `—` on empty data.
- [ ] `Avg response` stays as `—` with an aria-description / tooltip explaining undefined-over-0.
- [ ] `Pending: 0` unchanged.
- [ ] Vitest pin asserts both empty and populated KPI strips.
- [ ] No visual regression against the locked E-ops-dashboard design.

## May-touch

- `web/components/skycot/approvals/KpiStrip.tsx` (or wherever the KPI rendering lives)
- `web/lib/format/kpi.ts` (or extend) — add `formatKpi` helper
- `web/components/skycot/approvals/__tests__/KpiStrip.test.tsx` (new or extend)

## Out of scope

- Approvals page empty-state copy ("All clear / Your teammates have nothing waiting on you") — already in the locked design
- The `$ at risk` donut shape (covered by `fix-approvals-ops-dashboard-dollar-at-risk-donut-shape-implies-percent`)
- Hardcoded KPI fixtures (covered by `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)

## Effort

XS — ~20-40 LOC. ~30-45 min. P3 because no demo scene drills into the empty KPI strip; pure polish.
