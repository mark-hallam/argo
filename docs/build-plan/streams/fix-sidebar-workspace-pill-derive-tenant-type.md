# fix-sidebar-workspace-pill-derive-tenant-type

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-sidebar-workspace-pill-derive-tenant-type`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F21 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1 — Sidebar workspace pill always shows "Personal workspace" regardless of tenant type. Actively undermines the Claim 6 ("Multi-tenant by construction") narrative.**

## Root cause

Bottom-left of the app shell shows: workspace name (e.g. `demo+e2e`) over a fixed label `"Personal workspace"`. The label is hardcoded — an enterprise tenant, a team workspace, or any other tenant type would still see "Personal workspace".

Touch site: `web/components/skycot/AppShell.tsx` (the UserPill / WorkspaceLabel component).

Multi-tenant claim is one of the 6 MISSION load-bearing claims. The sidebar pill is in front of the user on every page. Even if the backend correctly scopes per tenant, the UI lies to the user about what kind of tenant they're in.

## Findings this brief closes

- whole-site-mvp-claims-coverage F21 [P1] — Sidebar workspace pill labels every tenant as "Personal workspace" regardless of type

## Fix recipe

1. **Read workspace type from tenant metadata**. The gateway likely exposes tenant metadata via `/api/v1/tenant` or as part of the session JWT. Touch:
   - `web/lib/api/tenant.ts` (or wherever tenant metadata is fetched)
   - The session bag / `useSession()` hook

2. **Render the appropriate label** in `web/components/skycot/AppShell.tsx`:
   - `tenant.type === "personal"` → "Personal workspace"
   - `tenant.type === "team"` → "Team workspace"
   - `tenant.type === "enterprise"` → "Enterprise workspace"
   - Unknown / missing → fall back to the tenant slug only (no fake label)

3. **If the gateway doesn't yet expose workspace type**: add the field. Touch:
   - `crates/argo-cli/src/handlers/gateway/routes_tenant_metadata_get.rs` (NEW or extend existing)
   - The `tenants` table schema (add `tenant_type` enum column via additive migration per `docs/skycot/operator/migrations.md`)
   - Default value: `"personal"` for any tenant created before the migration.

4. **Pin** with a test at `web/components/skycot/AppShell.test.tsx`:
   - Render with `tenant: {type: "personal"}` → assert label "Personal workspace".
   - Render with `tenant: {type: "team"}` → assert label "Team workspace".
   - Render with `tenant: {type: "enterprise"}` → assert label "Enterprise workspace".

## Acceptance criteria

- [ ] Sidebar pill displays the correct workspace-type label based on tenant metadata.
- [ ] Demo tenant (`demo+e2e`) shows "Personal workspace" (preserved behavior for that one).
- [ ] A team or enterprise tenant shows the right label.
- [ ] If tenant type is missing, the fallback is honest (slug only, no fake label).
- [ ] Test pin passes.

## May-touch

- `web/components/skycot/AppShell.tsx`
- `web/lib/api/tenant.ts`
- `crates/argo-cli/src/handlers/gateway/routes_tenant_metadata_get.rs` (if endpoint needs creating)
- `crates/argo-store/src/migrations/<NN>_tenant_type.sql` (NEW if column doesn't exist)
- `web/components/skycot/AppShell.test.tsx` (NEW or extend)

## Out of scope

- Multi-tenant theming (separate roadmap item)
- Workspace-type management UI (separate brief)
- Settings → Members page (separate brief `fix-settings-members-hydration-mismatch-unstyled`)

## Effort

~80-200 LOC (depending on whether the gateway change is needed). ~2-4 hours.
