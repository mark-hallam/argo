# fix-worker-detail-responsibilities-new-orphan-route-decision

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-responsibilities-new-orphan-route-decision`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-1-and-2-claim-1-and-2.md` F5.

🟢 **P3 — `web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx` exists as a route, but `WORKER_TAB_IDS` has no "responsibilities" or "schedules" entry. The route is reachable via direct URL but has no tab link or breadcrumb in the in-product UI. It's either a leftover from an earlier brief or a feature that needs to be exposed — currently it dangles, eating bundle size and creating an undiscoverable surface.**

## Root cause

`web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx` ships a working page (presumably a "create a new responsibility/schedule" form). `web/components/skycot/WorkerTabs.tsx:19-27` declares the canonical `WORKER_TAB_IDS = ['overview', 'activity', 'memory', 'skills', 'trust', 'escalation', 'settings']` — no `responsibilities` or `schedules`. No tab links to it; no breadcrumb back to the worker detail surface; no link from `DutiesCard.tsx`'s "+ Add a schedule" button (which mounts `AddResponsibilityDialog` inline, not via routing).

Three reasonable resolutions:
- **(a) Delete the route** if it's dead code from an earlier scaffold.
- **(b) Wire it to a "Schedules" tab** — but that's the scope of `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md` (existing brief), which would deprecate the dialog and reuse this route as the canonical create flow.
- **(c) Wire it to the existing dialog** — keep the dialog as the primary affordance, route to this page only when the dialog needs a "full screen edit" affordance (similar to how Linear has both a quick-add and a full-page issue creator). Lowest disruption.

This brief is the **decision brief** — pick one path, document the rationale, and execute. Recommend (a) for v1.0 demo (smallest surface area, removes an undiscoverable route), with the option to revisit (c) post-MVP if the dialog grows too complex.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F5 [P3] — Worker-detail page has a `responsibilities/new` route segment but no Schedules tab to land on

## Fix recipe

**Recommended path (a): delete the orphan route.**

1. `rm -rf web/app/(app)/workers/[workerId]/responsibilities/` (entire directory).
2. `rg 'responsibilities/new|responsibilities\b' web/app/ web/components/ web/lib/` to find any callers; expect zero (per audit; verify).
3. Update Playwright + Vitest tests if any reference the path; expect zero.

**Alternative path (b): wire it to a real Schedules tab.**

4. This becomes scope for `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md` — defer this brief to that one. If that brief lands first, close this one as no-op.

**Alternative path (c): keep route + wire it to the dialog as a "Full edit" affordance.**

5. Add a `<Link href="...">Edit on full page →</Link>` in `AddResponsibilityDialog.tsx` footer; the dialog page renders the route on a new tab/window or as a full-screen takeover. Update `DutiesCard.tsx` accordingly.

## Acceptance criteria

- [ ] `rg -l 'workers/\[workerId\]/responsibilities/new' web/` returns either zero (path a deleted) OR a complete set of callers from a Schedules tab (path b) OR a real link from the dialog (path c).
- [ ] No reachable-but-unlinked routes in `web/app/(app)/workers/[workerId]/**` (verify via `next-router-mock` test).
- [ ] If deleted: bundle-size check passes (the deleted route's chunk no longer ships).
- [ ] If kept: a Playwright walk reaches the route via the in-product nav (no direct URL needed).

## May-touch

- `web/app/(app)/workers/[workerId]/responsibilities/` (entire dir if path a)
- `web/components/skycot/WorkerTabs.tsx` (if path b — out of scope here)
- `web/components/skycot/worker/DutiesCard.tsx` + `AddResponsibilityDialog.tsx` (if path c)

## Out of scope

- The full Schedules tab implementation (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md`)
- Renaming "responsibilities" → "schedules" in code (operator-locked: code stays "duty"; UI says "schedule")

## Effort

XS — path (a) is ~20 LOC delete + verification. ~30 minutes. Path (b) is deferred. Path (c) is ~60 LOC. Default to (a).
