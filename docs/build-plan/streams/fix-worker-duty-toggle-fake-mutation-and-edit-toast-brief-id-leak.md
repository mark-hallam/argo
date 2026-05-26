# fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-1-and-2-claim-1-and-2.md` F8 + F9.

🔴 **P0 DEMO-GATE — Two camera-visible defects on the worker-detail Schedules card:**
**(F8)** The duty toggle is a fake — flips the UI switch but never hits the gateway. Camera sees a "real" toggle, next page-load reverts.
**(F9)** Clicking a duty body emits a toast with the literal string "Edit schedule ships in E4-h" — leaking the swarm brief-ID into the customer surface.

## Root cause

`web/components/skycot/worker/WorkerOverviewContent.tsx` ships two `console.warn`-as-callback stubs:

1. **Toggle stub at lines 87-92.** `onToggleDuty` is defined as:
   ```ts
   const onToggleDuty = (duty) => {
     console.warn(`mutation lands with E4-h: toggleDuty ${duty.id}`);
     // (no POST to gateway)
   };
   ```
   The duty switch's `onChange` is wired to this, so the UI control flips visually but no `PATCH /api/v1/workers/<id>/duties/<dutyId>` mutation fires. The next page-load fetches duties from `useDuties()` and re-renders the original state — the toggle "snaps back" silently.

2. **Edit toast at lines 78-81 / 175-182.** `onEditDuty` is defined as:
   ```ts
   const onEditDuty = (duty) => {
     setToast({ message: "Edit schedule ships in E4-h", testid: "overview-toast" });
   };
   ```
   Clicking a duty body shows a bottom-right toast card whose visible text contains the engineering brief-ID `E4-h`. The `data-testid="overview-toast"` doesn't matter — the on-screen string is the camera-visible defect.

Same root file, same root pattern (placeholder callbacks pending the E4-h brief). Bundling them so the fix lands together.

Per `feedback_no_theater_in_demo` (operator-locked 2026-05-24): the demo path must be real backend. Per `feedback_no_theater_in_demo` corollary: customer-facing surfaces must never leak engineering codes. F9 violates both.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F8 [P0] — Duty toggle is a fake — flips visual switch but no mutation persisted
- scene-1-and-2-claim-1-and-2 F9 [P0] — Clicking duty body emits brief-ID leak toast "Edit schedule ships in E4-h"

## Fix recipe

Three paths; sequence them.

**(a) Short-term (no theater, no leak) — disable the controls until the mutations exist.**

1. In `WorkerOverviewContent.tsx`, set the duty toggle's `disabled={true}` until the PATCH endpoint exists. Add a small tooltip "Schedule editing lands soon."
2. Drop the click handler on the duty body entirely (or convert to a `cursor-default`).
3. Remove the entire toast string referencing `E4-h`. If a toast must exist, the body says: "Schedule editing is coming. For now, you can disable + re-add."

**(b) Mid-term — wire the toggle to a real endpoint.**

4. Implement `PATCH /api/v1/workers/<id>/duties/<dutyId>` on the gateway (per existing sibling brief `fix-gateway-worker-duties-schedules-routes-404.md`).
5. SPA-side: replace the stub with `useToggleDuty(dutyId)` React Query mutation; optimistic update + rollback on error.
6. Wire the duty-body click to a real edit dialog (cohort with sibling brief `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md`).

**(c) CI guard.**

7. Add a CI grep: `grep -rE '\bE[0-9]+-[a-z]\b' web/components web/app` → exit 1 if matches. This is the same guard as `fix-worker-tabs-placeholder-bodies-leak-brief-ids-e4.md`; one CI lint, two violations covered.

## Acceptance criteria

- [ ] Duty toggle is either (a) disabled with a soft tooltip OR (b) wired to a real PATCH mutation that persists across reload.
- [ ] Clicking a duty body either does nothing OR opens a real edit dialog — but never shows a toast containing "E4-h" or any other brief-ID.
- [ ] CI grep guard catches new `E[0-9]+-[a-z]` user-facing strings.
- [ ] Vitest pin: clicking the duty toggle either does nothing (option a) OR fires the PATCH mock (option b); the page never renders the literal string "E4-h".

## May-touch

- `web/components/skycot/worker/WorkerOverviewContent.tsx` (stubs at 78-92, 175-182)
- `web/components/skycot/worker/DutiesCard.tsx` (toggle + body click props)
- `web/lib/hooks/use-duty-mutations.ts` (NEW — if option b)
- `crates/argo-cli/src/handlers/gateway/routes_worker_duties.rs` (if option b)
- Vitest test + CI guard

## Out of scope

- Full Schedules tab promotion (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md`)
- Card title rename "Responsibilities" → "Schedules" (covered by `fix-playwright-duty-to-schedule-vocab-completion.md`)
- Duty-add affordance (covered by sibling above)

## Effort

S (option a) or M (option b). Option (a) alone: ~30 LOC + tests, ~30-60 min. Option (b) end-to-end: ~150-300 LOC + endpoint, ~3-5 hours.

Recommend ship option (a) immediately to clear demo gate, file option (b) as the follow-up.
