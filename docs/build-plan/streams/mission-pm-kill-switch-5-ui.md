# Stream P2-12-5 — kill switch: web UI freeze button + confirm modal

**Owner**: claude.
**Depends on**: mission-pm-kill-switch-4-gateway MERGED.
**Authored against**: origin/main `6bcb9aeb` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-kill-switch-5-ui`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §11 P2 #12. Split slice 5/5 of parent `mission-pm-kill-switch` (BLOCKED 2026-05-19 — incomplete may-touch).

## Scope

The operator-facing UI for worker freeze/unfreeze on the worker detail page. Two pieces:

1. A red "Freeze worker" button in the worker profile page (`web/app/(app)/workers/[workerId]/page.tsx`).
2. A confirm modal that requires typing the worker name (or worker ID) to confirm — this is destructive UX, NOT a one-click action.

When frozen, the button flips to "Unfreeze" (single-click, no name-typing required — unfreeze is the safe direction).

The page also displays a banner at the top showing the freeze state when active: red banner with the reason, who froze it, and when.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `web/app/(app)/workers/[workerId]/page.tsx` — add the freeze banner (visible when frozen) + the Freeze/Unfreeze button. Pull current freeze state from a new server-side fetch.
- `web/app/(app)/workers/[workerId]/FreezeButton.tsx` (NEW) — the button + confirm modal client component. Imports a shared UI primitive for the modal if one exists; otherwise inline.
- `web/app/api/workers/[id]/freeze/route.ts` (NEW) — Next.js server route that proxies to `POST /api/v1/workers/{id}/freeze` on the gateway. Reads session, forwards auth, returns the gateway response.
- `web/app/api/workers/[id]/unfreeze/route.ts` (NEW) — same pattern for the unfreeze direction.
- `web/lib/api/workers.ts` (or wherever the worker API client lives — verify path before editing) — add `freezeWorker(id, reason)` and `unfreezeWorker(id)` helpers.
- `web/app/(app)/workers/[workerId]/FreezeButton.test.tsx` (NEW) — Vitest component test: (a) button renders red when not frozen; (b) clicking opens the modal; (c) typing the wrong name keeps confirm disabled; (d) typing the right name enables confirm; (e) when `isFrozen=true`, button flips to "Unfreeze" and bypasses the name-confirm.

## Files you must NOT touch

- `crates/**` — all backend slices are owned by siblings 1-4.
- Any other worker-page component beyond `FreezeButton.tsx` and the page.tsx integration point.

## Acceptance criteria

- [ ] Worker profile page shows a red banner with freeze reason + actor + timestamp when the worker is frozen.
- [ ] "Freeze worker" button is red, opens a confirm modal that requires typing the worker ID to enable the confirm button.
- [ ] On confirm + reason text, POSTs to the Next.js route, which proxies to the gateway; on 200 the page reloads and shows the frozen state.
- [ ] When frozen, button label flips to "Unfreeze worker" (no name-typing required for unfreeze).
- [ ] Without `worker.freeze` capability, button is hidden (server-side check before render — do NOT render-then-disable).
- [ ] Component tests pass.
- [ ] `pnpm lint && pnpm test && pnpm build` clean in `web/`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-kill-switch-5-ui`
- PR title: `feat(web): worker freeze button + confirm modal (mission-pm-kill-switch-5-ui)`

## Notes for the implementer

- BEFORE writing the button, look at an existing destructive-action button in the codebase — e.g. archive worker, delete account — and match its visual language. Don't invent a new red-button pattern.
- The confirm-modal pattern (type-the-name) should mirror GitHub's "delete repo" confirm flow: input is disabled-confirm until exact-match.
- Server-side capability check: in `page.tsx`, fetch the session's capabilities and pass `canFreeze: boolean` to the FreezeButton. Don't ship the button to the client at all if the user doesn't have the capability — server-side guard, not client-side disable.
- The freeze banner is `bg-red-500` / `text-white` / icon=ban — make it loud so operators don't miss it.
- Both Next.js routes are thin proxies. Don't do business logic in them. Auth header forwarding is the only non-trivial bit; mirror the pattern from any existing `/api/workers/[id]/*` route.

## Out of scope

- Backend slices (siblings 1-4 — all MERGED first).
- A general-purpose "destructive action confirm modal" primitive — if `FreezeButton.tsx` ends up duplicating logic from elsewhere, file a follow-up brief to extract the primitive, but do not block this brief on it.
- The audit-trail summary ("frozen 3 times in last 30 days") on the profile page — separate brief.
