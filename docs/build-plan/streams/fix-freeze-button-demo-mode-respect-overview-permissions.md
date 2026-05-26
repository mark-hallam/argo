# fix-freeze-button-demo-mode-respect-overview-permissions

**Owner**: claude or codex (fast-mode candidate, ~20 LOC).
**Authored against**: origin/main `9374b917d` on `2026-05-26` (re-authored T09-10 after codex BLOCKED v1 for wrong may-touch paths).
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-freeze-button-demo-mode-respect-overview-permissions`.
**Source**: evaluator round 83 (2026-05-26T08:10Z) — follow-up to PR #2393 (`fix-gateway-demo-mode-worker-permissions-grant-all` MERGED at 08:31Z).

🟢 **P3 — Cosmetic SPA polish after demo-mode permissions grant.** PR #2393 wired `compute_permissions_demo` so demo-minted principals receive `{can_freeze: true, can_edit: true, ...}` on `/api/v1/workers/<id>` overview. The freeze button on the worker overview should honor that `can_freeze` field so the button ungreys mid-recording.

## Actual current files (post codex-BLOCKED-v1 correction)

- `web/app/(app)/workers/[workerId]/FreezeButton.tsx` (the live component — App Router path)
- `web/app/(app)/workers/[workerId]/FreezeButton.test.tsx` (sibling vitest file)

## Acceptance criteria

- [ ] Audit `web/app/(app)/workers/[workerId]/FreezeButton.tsx` for all `disabled` branches.
- [ ] Confirm that whether the button is disabled is driven by the `permissions.can_freeze` field from the `/api/v1/workers/<id>` response (after #2393). Currently the file references `canSubmit` (line 193) — verify that's only the typed-id-confirm gate inside the dialog, not the outer button.
- [ ] If the outer "Freeze worker" button has a separate hardcoded `disabled` / hidden branch (e.g. tied to `worker.archived_at` or always-true), wire it through the same `permissions.can_freeze` truth source.
- [ ] Vitest test pin in `FreezeButton.test.tsx`: with `permissions={can_freeze: true}` props, button is enabled; with `{can_freeze: false}`, button is disabled with the existing aria-label / tooltip.

## May-touch

- `web/app/(app)/workers/[workerId]/FreezeButton.tsx`
- `web/app/(app)/workers/[workerId]/FreezeButton.test.tsx`
- (optional) parent component that passes props down — only if needed to thread the `permissions` field through

## Out of scope

- Backend permission logic (already shipped in #2393).
- Other action buttons on overview (Approve / Decline / etc.) — separate brief if similar issue.
- Refactoring `FreezeBanner.tsx` or other unrelated sibling files.

## Worker note

Tiny brief; the actual fix may be a one-line wire of `permissions?.can_freeze ?? false` to the existing button. Fast-mode candidate. Falls under operator-locked `--skip-clippy` fix-* / ops-* / eval-* auth after evaluator pre-verification.
