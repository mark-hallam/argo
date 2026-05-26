# fix-freeze-button-demo-mode-respect-overview-permissions

**Owner**: claude or codex (fast-mode candidate, ~20 LOC).
**Authored against**: origin/main `f208bcb70` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-freeze-button-demo-mode-respect-overview-permissions`.
**Source**: evaluator round 83 (2026-05-26T08:10Z) — `docs/skycot/evals/scheduled/2026-05-26/round-83-pr-2398-evidence-export-and-pr-2393-demo-mode-perms-pre-merge.md` (or equivalent). Follow-up to PR #2393 (`fix-gateway-demo-mode-worker-permissions-grant-all`).

🟢 **P3 — Cosmetic SPA polish after demo-mode permissions grant.** PR #2393 wires `compute_permissions_demo` so demo-minted principals receive `{can_freeze: true, can_edit: true, ...}` on `/api/v1/workers/<id>` overview. The freeze button on the worker overview now respects those permissions (ungreys mid-recording), BUT the same `FreezeButton.tsx` component may have an internal "disabled" branch keyed off something else (e.g. `worker.archived_at` or a hardcoded fallback), masking the new gate.

## Acceptance criteria

- [ ] Audit `web/components/skycot/worker/FreezeButton.tsx` for all `disabled` branches.
- [ ] Confirm the only `disabled` truth source is the `permissions.can_freeze` field returned by `/api/v1/workers/<id>` (after #2393 ships).
- [ ] If the component has a separate hardcoded `disabled` fallback, remove it OR wire it through the same permissions field.
- [ ] Vitest test pin: with `permissions: {can_freeze: true}` props, button is enabled; with `{can_freeze: false}`, button is disabled with the existing aria-label / tooltip.

## May-touch

- `web/components/skycot/worker/FreezeButton.tsx`
- `web/components/skycot/worker/FreezeButton.test.tsx` (or sibling test file)

## Out of scope

- Backend permission logic (already shipped in #2393).
- Other action buttons on overview (Approve / Decline / etc.) — separate brief if similar issue.

## Worker note

Tiny brief; the actual fix may be a one-line remove of a hardcoded `disabled` OR a one-line wire of `permissions?.can_freeze ?? false`. Fast-mode candidate. Falls under operator-locked `--skip-clippy` fix-* / ops-* / eval-* auth after evaluator pre-verification.
