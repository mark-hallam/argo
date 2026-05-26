# fix-worker-hero-overflow-menu-pause-disable-archive-stub

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-hero-overflow-menu-pause-disable-archive-stub`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-1-and-2-claim-1-and-2.md` F16.

🟦 **P2 — `/workers/<id>` hero overflow menu ("…") exposes Pause / Disable / Archive actions. Clicking any one fires `console.warn('worker hero action <action> pending wiring (E3-b/E4-f)')`. No toast, no error visible to the user. Camera unlikely to click but a curious viewer playing back the demo might.**

## Root cause

`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:131-137`:

```ts
const handleHeroAction = (action) => {
  console.warn(`worker hero action ${action} pending wiring (E3-b/E4-f)`);
};
```

The `WorkerHero` "…" button opens a Radix dropdown with three items: Pause, Disable, Archive. Each is wired to `handleHeroAction(<verb>)`. Clicking does nothing visible to the user — the only signal is a dev-console warn (also leaks "E3-b/E4-f" brief-IDs to anyone with DevTools open).

Per `feedback_no_theater_in_demo`: a customer-facing button that does nothing is theater. Two clean fixes: hide the menu until wired, or wire the actions.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F16 [P2] — `WorkerHero` pause/disable/archive overflow-menu is a console.warn stub leaking brief-IDs

## Fix recipe

**Path 1 — short-term, hide the menu.**

1. In `WorkerLayoutShell.tsx`, gate the overflow-menu rendering on a flag (default false): `const heroActionsEnabled = false; // TODO wire when E3-b/E4-f land`.
2. Remove the `handleHeroAction` stub entirely.
3. Vitest pin: `WorkerHero` does NOT render an overflow menu when `heroActionsEnabled` is false.

**Path 2 — long-term, wire the actions.**

4. Implement gateway endpoints:
   - `POST /api/v1/workers/<id>/pause` — sets worker status to `paused`, emits audit row.
   - `POST /api/v1/workers/<id>/disable` — same with `disabled`.
   - `POST /api/v1/workers/<id>/archive` — moves worker to soft-deleted state, emits audit row.
5. SPA: wire each menu item to its mutation. Show a confirmation dialog for Disable + Archive (Pause is reversible, less destructive).
6. After mutation: show toast "{name} paused / disabled / archived". Optimistic update.
7. Cross-tenant binding test per Claim 6 invariant.

Recommend Path 1 immediately for demo gate; Path 2 as the follow-up (separate brief if not bundled).

8. Apply the same CI grep guard as `fix-worker-tabs-placeholder-bodies-leak-brief-ids-e4.md` to catch any new `E[0-9]+-[a-z]` strings in user-visible components.

## Acceptance criteria

- [ ] `WorkerHero` overflow menu either (a) does not render OR (b) every item fires a real mutation + toast.
- [ ] No code in `web/app/(app)/workers/**` contains the string `E3-b` or `E4-f`.
- [ ] Vitest pin asserts menu visibility matches the flag.
- [ ] CI grep guard catches new brief-IDs.

## May-touch

- `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx` (lines 131-137)
- `web/components/skycot/WorkerHero.tsx` (menu rendering)
- `crates/argo-cli/src/handlers/gateway/routes_worker_lifecycle.rs` (NEW — option 2)
- Vitest test + CI guard

## Out of scope

- Worker-tab placeholder bodies (covered by `fix-worker-tabs-placeholder-bodies-leak-brief-ids-e4.md`)
- Duty toggle stub (covered by `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.md`)
- Full worker lifecycle policy (separate post-MVP design)

## Effort

XS (Path 1 — ~20 LOC). M (Path 2 — ~300 LOC across gateway + SPA + tests).
