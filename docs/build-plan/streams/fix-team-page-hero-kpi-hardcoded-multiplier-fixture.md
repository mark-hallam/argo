# fix-team-page-hero-kpi-hardcoded-multiplier-fixture

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-hero-kpi-hardcoded-multiplier-fixture`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-1-and-2-claim-1-and-2.md` F15.

🟦 **P2 — `/team` hero KPIs are derived from `workers.length * 187` (weeklyActions) and `workers.length * 12` (hoursSaved). The page shows "5 teammates, one team. 4 working right now · 935 actions this week · saving you ~60 hours." All three numbers are synthetic multipliers. Per function-vs-dressing this is dressing-side IF the runbook doesn't narrate the exact numbers; but if Scene 1's voiceover reads "935 actions this week", that's a function-side claim built on fiction.**

## Root cause

`web/app/(app)/team/page.tsx:151-152`:
```ts
const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;
```

The "4 working right now" figure is similarly hardcoded or derived from `workers.filter(w => w.status === "active").length` against fixtures.

The numbers serve their job as visual dressing — they're plausible, they don't draw attention — but they're not connected to any real telemetry. A buyer who asks "where do these numbers come from?" would be told "it's a fixture multiplier", which erodes the audit-trust narrative the rest of the product is selling.

Per memory `feedback_no_theater_in_demo`: dressing-side fixtures are OK; function-side narrations are not. The numbers here are visible above the fold on `/team` (which `/workers` redirects to). If Scene 1's narration reads "935 actions this week" aloud, this becomes function-side and the brief lifts to P1.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F15 [P2] — Hero KPI math on `/team` is hardcoded multiplier — `workers.length * 187` / `* 12`

## Fix recipe

Two paths.

**(a) Short-term — make the dressing honest.** Replace the multipliers with a single static-text variant that doesn't claim a count:
1. `web/app/(app)/team/page.tsx:151-152` — delete the multipliers.
2. Replace the sub-header text with "Your AI teammates. Working on the schedules you set." (no number, no fictional claim).
3. Audit feed (separate page) is where real action counts live; link from the team hero with "See activity →".

**(b) Long-term — wire to a real KPI rollup endpoint.**
1. Implement `GET /api/v1/team/kpi-rollup?range=7d` on the gateway:
   - `weeklyActions` = `COUNT(audit_rows WHERE timestamp >= now - 7d AND tenant_id = <t>)`
   - `hoursSaved` = derived from a per-vertical "minutes saved per action" coefficient × weeklyActions / 60. (Coefficient lives in the vertical manifest; default 5 min/action.)
   - `workingRightNow` = workers with status == "active" in the team_workers join.
2. SPA: `useTeamKpi()` hook + render real numbers.
3. Tenant-scoped per existing Claim 6 isolation.

**Recommend (a) now (XS effort, clears the dressing gap before MP4 record) + file (b) as the post-MVP follow-up.**

4. Vitest pin: hero text does NOT contain a hardcoded `187` / `12` constant.

## Acceptance criteria

- [ ] `/team` hero displays no fictional action counts; OR (b) displays real counts from a tenant-scoped gateway endpoint.
- [ ] `grep -E 'workers\.length \* [0-9]' web/app/(app)/team/page.tsx` returns no matches.
- [ ] Vitest pin passes.
- [ ] If (b): cross-tenant negative test on KPI rollup endpoint.

## May-touch

- `web/app/(app)/team/page.tsx` (lines 151-152 + hero copy)
- `web/components/skycot/TeamHero.tsx` (if hero is extracted)
- `crates/argo-cli/src/handlers/gateway/routes_team_kpi.rs` (NEW — option b)
- Vitest test

## Out of scope

- Approvals dashboard KPIs (covered by `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts.md`)
- Home page approvals strip (covered by `fix-home-approvals-kpi-strip-hardcoded-7d-stats.md`)
- "4 working right now" badge per worker card (different visual element)

## Effort

XS (option a — ~10 LOC) or M (option b — ~150-250 LOC).
