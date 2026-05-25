# fix-team-page-hero-hardcoded-weekly-actions-hours-saved

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-hero-hardcoded-weekly-actions-hours-saved`.
**Source**: e2e-ux-audit Agent 1 finding F4 + Agent 2 finding F7 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🔴 **P0 DEMO-GATE — first frame after hire LIES about real activity. Defeats the audit-first / no-theatre thesis.**

## Root cause

`web/app/(app)/team/page.tsx:147-148`:

```ts
const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;
```

The team hero subtitle reads:

> "0 working right now · 187 actions this week · saving you ~12 hours"

…but the gateway's `/api/v1/team` for Maya returns `headline.bullets: ["No activity yet."]`, `last_active_at: null`, `status: "idle"`. Maya has zero real actions. The hero KPI strip fabricates 187 actions × 12 hours per worker.

Agent 2 additionally notes the hero copy ("Your first teammate is working") directly contradicts the KPI ("0 working right now") in the same frame.

Per `feedback_no_theater_in_demo` (operator-locked 2026-05-24) the v1.0 demo MUST be REAL not mocks. This is the canonical example of dressing pretending to be function.

## Findings this brief closes

- Agent 1 F4 [P0] — `/team` hero hardcodes "187 actions/wk, 12 hours saved" per worker
- Agent 2 F7 [P1] — team-page hero internally contradicts the worker count ("working" hero copy vs "0 working" KPI)

## Fix recipe

1. Replace the `workers.length * 187 / 12` math with values derived from the real audit feed. The gateway's `/api/v1/team` response already returns `headline.bullets` per worker — promote that to a tenant-level aggregate, or sum real action counts from `/api/v1/audit?range=7d` (already wired for the audit page).
2. When real activity == 0 (e.g. brand-new tenant just post-hire), the hero copy should reflect that honestly:
   - Hero title: "Your team is ready" (not "Your first teammate is working")
   - Hero subtitle: "Just hired — your first teammate hasn't acted yet" or "0 actions yet · waiting on first task"
3. Make the "working right now" pill reactive: derive from `worker.status === "working"` count, not from worker count.
4. If a real 7-day rollup endpoint doesn't exist yet, prefer **derive client-side from `useAuditList({range:'7d'})`** over fabricating — even if the number is small or zero, it's REAL.

## Acceptance criteria

- [ ] `/team` hero shows REAL action counts (sourced from audit feed or a gateway aggregate endpoint), not `workers.length * 187`.
- [ ] When `workers.length > 0` but `working === 0`, hero copy says "Your team is ready" (or similar idle-honest phrasing), NOT "Your first teammate is working".
- [ ] curl-test: against the current demo tenant (Maya only, 0 actions, idle), the hero subtitle does NOT say "187 actions this week" or "12 hours saved".
- [ ] Regression test pinning the derivation so the next refactor doesn't reintroduce hardcoded constants.

## May-touch

- `web/app/(app)/team/page.tsx` (the `weeklyActions` / `hoursSaved` block, hero copy)
- `web/lib/api/team.ts` (if a new aggregate field needs to come from the gateway)
- `crates/argo-cli/src/handlers/gateway/routes_team*.rs` (optional — if adding `weekly_action_count` to the team response)

## Out of scope

- TeammateCard LEVEL hash (separate brief: `fix-worker-personality-and-level-derive-from-real-data`)
- Approvals KPI fabrications (separate brief: `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)

## Effort

~30-60 LOC. ~30-45 min.
