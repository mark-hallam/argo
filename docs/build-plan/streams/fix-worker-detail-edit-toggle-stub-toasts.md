# fix-worker-detail-edit-toggle-stub-toasts

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-edit-toggle-stub-toasts`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-1-and-2-claim-1-and-2.md` F7 [P1].

🟠 **P1 — Clicking the body of a Responsibility row on the `/workers/{id}/overview` page fires a stub announcement "Edit schedule ships in E4-h" instead of opening an editor. The Toggle button visually flips but `onToggleDuty` is `console.warn` only — the duty's actual status never changes. Only the "Add responsibility" CTA works. So in the demo runtime the operator literally cannot edit a worker's schedules from the UI.**

## Root cause

`web/components/skycot/worker/WorkerOverviewContent.tsx:78-92` wires the row click handlers to placeholder announcements:

```tsx
const onEditDuty = (dutyId: string) => {
  setAnnouncement("Edit schedule ships in E4-h");
};
const onToggleDuty = (dutyId: string) => {
  console.warn("toggle duty", dutyId);
};
```

The gateway endpoint exists at `POST/PATCH /api/v1/workers/<id>/duties/<duty-id>` (verify in `crates/argo-cli/src/handlers/gateway/routes_worker_duties.rs`). The SPA's `web/lib/api/duties.ts` likely already has the client helpers (the sister brief `fix-worker-detail-responsibilities-schedules-tab-end-to-end` references `runDutyNow`). So the function exists; the page just doesn't wire it.

Two policy options:
- (a) Wire Edit + Toggle to real mutations.
- (b) Visually disable Edit + Toggle in demo mode so the operator doesn't accidentally click them on camera.

Recommended: (a). Toggle is trivial to wire (PATCH duty.enabled). Edit needs a modal but the gateway endpoint already exists for duty mutations.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F7 [P1] — Edit Responsibility row fires a "ships in E4-h" stub toast (not a real editor); Toggle button visually flips but onToggleDuty is console.warn only

## Fix recipe

1. **Wire Toggle to a real mutation** at `web/components/skycot/worker/WorkerOverviewContent.tsx`:
   ```tsx
   const { mutate: toggleDuty } = useToggleDuty(workerId);
   const onToggleDuty = (dutyId: string, nextEnabled: boolean) => {
     toggleDuty({ dutyId, enabled: nextEnabled });
   };
   ```
   Hook `useToggleDuty` lives at `web/lib/hooks/use-duties.ts` (NEW or extend existing). On success, refetch the overview query (`useWorkerOverview(workerId)`).

2. **Wire Edit to a real modal**:
   - Build `web/components/skycot/worker/EditDutyDialog.tsx` (NEW) — a modal that takes a `Duty` and renders editable fields (name, cron expression, description, toggle).
   - On save, calls `PATCH /api/v1/workers/<id>/duties/<duty-id>` via `useUpdateDuty`.
   - On close, refetches the overview.
3. **Replace the stub announcement** at `WorkerOverviewContent.tsx:78-92`:
   ```tsx
   const [editingDuty, setEditingDuty] = useState<Duty | null>(null);
   const onEditDuty = (duty: Duty) => setEditingDuty(duty);

   {editingDuty && (
     <EditDutyDialog
       duty={editingDuty}
       workerId={workerId}
       onClose={() => setEditingDuty(null)}
     />
   )}
   ```
4. **Surface error states** — if the mutation fails (gateway 4xx/5xx), show a destructive toast "Couldn't save schedule. Try again."
5. **Add a Playwright spec** `web/playwright/worker-detail/edit-and-toggle-duty.spec.ts`:
   - Open `/workers/{id}/overview` for Mariah.
   - Click the Toggle on a duty; assert the duty's enabled state flips AND a network PATCH was made.
   - Click the Edit affordance; assert the modal opens.
   - Edit a field, click Save; assert the row reflects the change.

## Acceptance criteria

- [ ] Toggle on a duty row fires a real gateway PATCH and the UI reflects the persisted state.
- [ ] Edit on a duty row opens a real editor modal (no stub announcement).
- [ ] Save in the editor persists changes via the gateway.
- [ ] Refresh of the page retains the persisted state.
- [ ] New Playwright spec passes.
- [ ] No regression on the "Add responsibility" CTA (existing real-mutation path).

## May-touch

- `web/components/skycot/worker/WorkerOverviewContent.tsx`
- `web/components/skycot/worker/EditDutyDialog.tsx` (NEW)
- `web/lib/hooks/use-duties.ts` (extend with toggle + update hooks)
- `web/lib/api/duties.ts` (verify client helpers exist; add toggle/update if missing)
- `crates/argo-cli/src/handlers/gateway/routes_worker_duties.rs` (verify PATCH endpoint accepts the fields edited here)
- `web/playwright/worker-detail/edit-and-toggle-duty.spec.ts` (NEW)

## Out of scope

- Run-now affordance (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end`).
- The Responsibilities → Schedules naming (covered by `fix-playwright-duty-to-schedule-vocab-completion`).
- Adding new duties via the CTA (already works).

## Effort

~250-400 LOC (modal + hooks + spec). ~5-8 hours. P1 — the operator-facing edit affordance is visibly clickable but fakes the action; honesty issue.
