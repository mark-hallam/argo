# fix-settings-workspace-tenant-identity-surface

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-workspace-tenant-identity-surface`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-6-claim-6-trust-and-approvals.md` F7 [P1].

🟠 **P1 — `/settings/workspace` is entirely hardcoded React; the page shows the same "SKYCOT Demo" / "Marketing operations" / "Australia/Melbourne" / "owner@skycot.example" content for every session, regardless of tenant. `useSkycotSession()` exposes `tenantId` (e.g. `tenant_acmeco` in demo mode) but it's not consumed anywhere on this page or in any other customer-facing surface. The operator cannot demonstrate "you are scoped to one tenant" — Claim 6's multi-tenant ergonomics are invisible.**

## Root cause

`web/app/(app)/settings/workspace/page.tsx` is a static React component. Every visible field is baked in:
- Display name = "SKYCOT Demo"
- Vertical = "Marketing operations"
- Timezone = "Australia/Melbourne"
- Billing contact = "owner@skycot.example"

The session hook at `web/lib/hooks/use-skycot-session.ts` (or equivalent) already exposes `tenantId` and `workspaceName`. The page just doesn't read them. There's also no tenant chip / id in the user-pill popover or anywhere else.

Claim 6 (multi-channel + multi-tenant) is LIVE backend (per CLAUDE.md), and the tenant scoping is enforced at the SQL layer. But for a buyer to understand "this product is multi-tenant", they need a UI signal. Today they can't see one.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F7 [P1] — `/settings/workspace` is entirely hardcoded; no tenant identity surfaced anywhere

## Fix recipe

1. **Surface the tenant id + workspace fields on `/settings/workspace`**:
   - At `web/app/(app)/settings/workspace/page.tsx`, replace the hardcoded values with reads from `useSkycotSession()` (or the equivalent server-component data fetch).
   - Add a read-only "Workspace ID" row: `<code>tenant_<slug></code>` with a click-to-copy affordance.
   - Add "Workspace name", "Display name", "Vertical" as inputs that read defaults from session.
   - Save handler calls `PATCH /api/v1/workspace` (existing endpoint — verify) on submit.

2. **Add a tenant chip in the user-pill popover** at `web/components/skycot/UserPill.tsx` (or wherever):
   - Below the user name, show `<span class="text-xs text-muted">Workspace · {tenantId}</span>`.
   - Tooltip with the full tenant id.

3. **Wire the tenant resolver into the page** if not already done:
   ```tsx
   const { tenantId, workspaceName, vertical, timezone } = useSkycotSession();
   ```
   Fall back to operator-facing strings ("Loading…" / "—") only if session is still hydrating.

4. **Add a Playwright spec** `web/playwright/settings/workspace-shows-tenant-id.spec.ts`:
   - Open `/settings/workspace`.
   - Assert the "Workspace ID" row is visible AND contains a non-empty value.
   - Assert the user-pill popover contains a tenant chip.

## Acceptance criteria

- [ ] `/settings/workspace` displays the actual `tenantId` from the session.
- [ ] Workspace name + vertical + timezone fields default to the session values, not hardcoded "SKYCOT Demo" copy.
- [ ] The user-pill popover surfaces the tenant id as a chip.
- [ ] Click-to-copy works on the Workspace ID row.
- [ ] New Playwright spec passes.
- [ ] No regression on the settings shell layout (D top-tabs lock per `project_settings_page_design_locked`).

## May-touch

- `web/app/(app)/settings/workspace/page.tsx`
- `web/components/skycot/UserPill.tsx` (or equivalent)
- `web/lib/hooks/use-skycot-session.ts` (verify shape; should not need changes)
- `web/playwright/settings/workspace-shows-tenant-id.spec.ts` (NEW)

## Out of scope

- Full tenant switcher (multi-workspace UX) — out of v1 scope.
- Settings shell double-h1 (covered by `fix-settings-shell-double-h1-and-danger-zone-redirect`).
- Settings sub-page title suffixes (covered by `fix-settings-sub-pages-title-suffix-consistency`).
- Members / providers / billing surfaces.

## Effort

~80-150 LOC + 1 spec. ~3-5 hours. P1 because Claim 6 is invisible in the SPA today and Scene 6 narrates multi-tenancy as a value-prop.
