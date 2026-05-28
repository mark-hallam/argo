# rebase-fix-settings-workspace-tenant-identity-surface

**Owner**: claude.
**Authored against**: origin/main `a5df90c41` on `2026-05-29`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh rebase-fix-settings-workspace-tenant-identity-surface`.
**Supersedes**: PR #2417 (closed for re-author 2026-05-29) + original brief `streams/fix-settings-workspace-tenant-identity-surface.md`.

🔴 **P1 — Tenant identity not surfaced on /settings/workspace + user-pill chip. Re-implement against current main; the original PR #2417 conflicts irretrievably with the controlled-input rewrite of /settings/workspace landed in #2452.**

## What changed on main since PR #2417 was authored

PR #2452 (`d727e8c0d` — "fix(settings): wire workspace + providers + notifications panels") completely rewrote `/settings/workspace/page.tsx` as a controlled-input mutating page:
- Now uses `useState` for `persisted` + `draft`, mounts a fetch via `fetchWorkspaceSettings()`
- Has a Save bar that flips state optimistically + reverts on error
- Backed by new `web/lib/api/settings.ts` (fetchWorkspaceSettings / updateWorkspaceSettings) that hits a 404-fallback contract
- All inputs (display name, description, default vertical, timezone, billing contact) are now editable

PR #2417's approach (read-only session-derived defaults + click-to-copy tenant id row) cannot be merged directly into this new shape. The two PRs answer different design intents: #2452 treats /settings/workspace as a mutating panel; PR #2417 treats it as read-only identity surface.

## What this brief MUST ship

The unique value of the original PR #2417 is:
1. **Tenant ID surfaced on /settings/workspace** with a click-to-copy affordance. Today the page does not show `tenantId` at all.
2. **Tenant chip in UserPill popover** beneath the email — visible from anywhere in the SPA.

Both are still load-bearing for Claim 6 ("Multi-channel + multi-tenant") demo visibility.

**Implementation against the new shape**:
1. In `web/app/(app)/settings/workspace/page.tsx` (the new controlled-input page), add a **read-only** Workspace ID row at the top of the form (above the editable fields). Pull `tenantId` from `useSkycotSession()`. Use the same `.settings-field` shell + a small "Copy" affordance (`navigator.clipboard.writeText`). Em-dash fallback when session is null.
2. In `web/components/skycot/UserPill.tsx`, add a tenant chip beneath the email line in the popover. Same source (`useSkycotSession()`). Click copies the tenant id.
3. Do NOT touch the Save bar logic or the controlled-input flow in #2452. The Workspace ID row sits beside, not inside, the mutating panel.

## Acceptance criteria

- [ ] `bash scripts/check-brief-freshness.sh rebase-fix-settings-workspace-tenant-identity-surface` passes.
- [ ] `/settings/workspace` displays a read-only Workspace ID row showing the actual `tenantId` from the session, with a click-to-copy affordance.
- [ ] Em-dash fallback when unauthenticated.
- [ ] UserPill popover shows a tenant chip (`appshell-user-tenant-chip`) beneath the email.
- [ ] Click-to-copy works on both surfaces (vitest assertions for both).
- [ ] No regression on the new controlled-input flow from #2452 (the Save bar, fetch-on-mount, optimistic update — all still green).
- [ ] No regression on the D top-tabs settings shell layout.
- [ ] New tests pinned at `web/tests/settings-workspace-tenant-id.test.tsx` + Playwright spec at `web/tests/e2e/settings-workspace-tenant-id.spec.ts` (the paths the original PR proposed).

## May-touch

- `web/app/(app)/settings/workspace/page.tsx` (additive — Workspace ID row + click-to-copy; do NOT replace the controlled-input flow)
- `web/components/skycot/UserPill.tsx` (tenant chip)
- `web/tests/settings-workspace-tenant-id.test.tsx` (NEW)
- `web/tests/e2e/settings-workspace-tenant-id.spec.ts` (NEW)
- `web/__tests__/settings/page.test.tsx` (if the new row breaks an existing assertion, update; otherwise leave alone)

## Out of scope

- Touching the Save bar / controlled-input flow from #2452.
- Wiring `/api/v1/workspace` PATCH (still out of scope per #2452's posture).
- Theme-picker work (already shipped via #2368).

## Effort

~80-120 LOC. ~1-2 hours.

## Source

- Original brief: `streams/fix-settings-workspace-tenant-identity-surface.md`
- Closed PR: #2417 (closed 2026-05-29 as REAL-CONFLICT-NEEDS-REAUTHOR)
- Merged PR that introduced the conflict: #2452 (`d727e8c0d`)
- UX audit: `docs/skycot/ux-audits/2026-05-26-2152/scene-6-claim-6-trust-and-approvals.md` F7
