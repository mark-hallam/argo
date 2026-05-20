# web-worker-detail-escalation-and-trust-chart — add /workers/[id]/escalation + trust-over-time chart

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-worker-detail-escalation-and-trust-chart`.
**Priority**: P1 — MISSION.md item #2 promises both surfaces and neither exists today; wave-1 trust+safety flagged both as gaps.
**Sizing**: ~400-600 LOC (new escalation route + new chart component + seed data + tests).

## Why this brief exists

From the wave-1 trust+safety audit:

> No `/workers/[id]/escalation` route and no trust-score-over-time
> chart. MISSION.md item #2 names escalation; the UI has none. The
> "trust ratchet earns trust over time" claim has no time-series
> visualisation.

The locked worker-detail design (memory:
`project_worker_detail_design_locked.md`) implies escalation rules
visible on each profile. The "trust ratchet" promise is on /landing
+ /pricing + /compare as a differentiator. Without time-series
evidence, the claim is unsupported on camera.

## Goals

1. New route: `/workers/[workerId]/escalation/page.tsx` with:
   - Current escalation rules (e.g. "Mariah escalates on: price drop > X%, edited-photo detection, multiple competing offers")
   - When-to-escalate matrix (action × condition → who-it-routes-to)
   - History of escalations triggered (with audit-row links)
2. New chart: trust-grade over time. Renders on `/workers/[workerId]/overview` below the personality card OR on a sibling tab `/trust`.
3. Use Recharts (already in package.json — check) or a simple SVG line chart if no chart dep.
4. Honour `prefers-reduced-motion` on any chart animation.

## Steps

1. Confirm chart library: `grep -r "recharts\|d3-shape\|victory" web/package.json`. If none, choose minimal-dep approach (raw SVG path + axis labels).
2. Read `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx` to understand tab structure + add a new tab.
3. Create `web/app/(app)/workers/[workerId]/escalation/page.tsx` (server component if static; client if interactive).
4. Build `<TrustOverTimeChart>` component under `web/components/skycot/workers/`.
5. Add seed: time-series trust-grade data per worker in `web/lib/sample-worker-trust-history.ts` (last 90 days, hourly buckets).
6. Wire the new tab into the worker-page nav (matches existing tab pattern).

## may-touch

- New: `web/app/(app)/workers/[workerId]/escalation/page.tsx`
- New: `web/app/(app)/workers/[workerId]/escalation/layout.tsx` (metadata)
- New: `web/components/skycot/workers/TrustOverTimeChart.tsx`
- New: `web/components/skycot/workers/EscalationMatrix.tsx`
- New: `web/lib/sample-worker-trust-history.ts`
- `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx` (add new tab)
- `web/components/skycot/WorkerTabs.tsx` (add escalation tab)

## must-NOT-touch

- `web/app/(app)/workers/[workerId]/overview/page.tsx` (existing tab; do not refactor)
- Trust-grade API + identity store (out of scope; chart reads from seed for v1)

## Acceptance criteria

- [ ] `/workers/wkr_01J0MARIAH/escalation` returns 200 with at least 3 escalation rules + a when-to-escalate matrix + a history table.
- [ ] Trust-over-time chart renders (line or area) with 90 days of data, hover shows date+value.
- [ ] Tab nav shows the new "Escalation" tab on every worker page.
- [ ] Reduced-motion guard: no animated transition when the OS preference is set.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Seed data is deterministic so the demo reproduces identically.

## References

- Audit: `docs/skycot/audit/trust-safety/2026-05-20-wave-1-trust-safety-audit.md`
- MISSION.md item #2 ("escalation")
- Operator memory `project_worker_detail_design_locked.md`
