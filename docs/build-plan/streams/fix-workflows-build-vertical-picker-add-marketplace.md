# fix-workflows-build-vertical-picker-add-marketplace

**Owner**: claude or codex.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-build-vertical-picker-add-marketplace`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F7 [P1]).

🟡 **P2 — Workflow-builder vertical picker at `web/app/(app)/workflows/build/page.tsx:4-10` enumerates 5 verticals (`sales / support / gtm-engineer / people-ops / finance`) but omits `marketplace`, the vertical the canonical Scene-5 workflow `customer_research_followup` is seeded under. So if the operator wants to author a new marketplace workflow in the canvas, the picker silently refuses. The list also drifts from the gateway-side truth at `/api/v1/verticals` — should be a single source of truth.**

## Root cause

`web/app/(app)/workflows/build/page.tsx:4-10`:

```ts
const VERTICALS = [
  { id: "sales", label: "Sales" },
  { id: "support", label: "Support" },
  { id: "gtm-engineer", label: "GTM Engineer" },
  { id: "people-ops", label: "People Ops" },
  { id: "finance", label: "Finance" },
] as const;
```

The canonical Scene-5 workflow `customer_research_followup` is seeded under vertical `marketplace` (`crates/argo-demo-seed/src/workflow.rs:26`). Existing seed workflows like `post_listing` are under `gtm-engineer`. The hard-coded picker doesn't know about `marketplace`.

Two problems:

1. **The picker is hard-coded.** Future verticals (e.g. `recruiting`, `pitch-agent` per Track P) won't appear without an SPA edit.
2. **It drifts from the gateway.** `/api/v1/verticals` SHOULD be the canonical list. Today that endpoint returns `{"verticals":[]}` per the audit, so even sourcing from it wouldn't help without backend seeding — but the dual-source structure means the picker can lie when the seed is healthy.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F7 [P1] — Builder vertical picker omits `marketplace` — the vertical the canonical Scene-5 workflow lives under

## Fix recipe

Two-tier fix; ship both:

1. **Immediate fix — add `marketplace` to the hardcoded list**:
   - `web/app/(app)/workflows/build/page.tsx:4-10` — add `{ id: "marketplace", label: "Marketplace" }` between `support` and `gtm-engineer` (or wherever alphabetical / dept-grouping puts it).
   - Optionally add `recruiting`, `pitch-agent` (per Track P) at the same time if the operator-locked list covers them.

2. **Structural fix — source from gateway**:
   - Change the build page to call `GET /api/v1/verticals` at mount-time, with a fallback to the hardcoded list when the gateway returns empty / 404 (covered by `mission-gateway-add-api-v1-verticals-cors-mounted-discovery` for the gateway side — verify status before this brief lands).
   - Cache the response for the session.
   - When the demo-seed populates verticals, the picker auto-reflects.

## Acceptance criteria

- [ ] Open `/workflows/build` — the vertical picker lists `marketplace` as a selectable option.
- [ ] Clicking the `marketplace` tile routes the user to `/workflows/marketplace/<name>/build` (matches the URL shape the canonical seed expects).
- [ ] (Structural-fix) The picker dynamically updates when the gateway returns a new vertical via `/api/v1/verticals`.
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run app/\(app\)/workflows/build` passes; add a regression that asserts at least `marketplace` is in the rendered tile list.

## May-touch

- `web/app/(app)/workflows/build/page.tsx` (~5 LOC immediate fix; ~40 LOC structural fix)
- `web/lib/api/verticals.ts` (new file if structural fix; ~50 LOC)
- One vitest spec under `web/app/(app)/workflows/build/__tests__/`

## Out of scope

- Seeding the gateway's `/api/v1/verticals` endpoint with real data (covered by `mission-gateway-add-api-v1-verticals-cors-mounted-discovery`)
- The canonical `customer_research_followup` workflow itself (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Builder palette MCP / CU step kinds (same brief)

## Effort

S. ~10 LOC immediate fix; ~80 LOC structural fix + test. ~30 min - 2 hours.
