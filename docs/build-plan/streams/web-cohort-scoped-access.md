# web-cohort-scoped-access — cohort-scoped RBAC UX in /settings/members

**Owner**: claude. **Sizing**: ~250-400 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-cohort-scoped-access`.
**Priority**: P2 — backend already supports cohort-scoped capabilities (capability gates in `argo-rbac`). Today /settings/members exposes workspace-level roles only (owner / admin / user / guest). This brief surfaces cohort scoping so e.g. "Diego only sees Sales cohort workers" is configurable from the UI.

## Scope

### 1. Update /settings/members member list

Each member row currently shows: name + email + role pill. Add:

- **Cohort scope** column:
  - "All cohorts" pill (default for owner/admin)
  - Or chip list of explicit cohorts (e.g. "Sales · Recruiting")
  - Truncate at 3 chips + "+N more" with hover tooltip listing the rest.
- Edit button → opens an Edit Member modal.

### 2. Edit Member modal

- Heading: "Edit access for <Member Name>".
- Role selector (existing): owner / admin / user / guest.
- **NEW: Cohort scope selector**:
  - Toggle: "All cohorts" (default for owner/admin) vs "Specific cohorts".
  - If Specific cohorts: multi-select chip-picker of all workspace cohorts (use existing cohort hook).
  - Helper text: "Member can only see workers, workflows, and audit entries belonging to selected cohorts."
- Save button → PATCH to gateway `/api/v1/members/<id>` with `{ role, cohort_scope: [<cohort-id>] | null }`. If endpoint doesn't exist, web brief authors against the documented contract.

### 3. Invite-member flow update

The existing "Invite member" modal should also surface the cohort-scope picker so invites land with scope already set. Default = All cohorts.

### 4. Audit integration

Audit entries when:
- A member's cohort scope changes ("Access changed: cohort scope set to [Sales]")
- A member is denied access due to cohort scope ("Cohort-scope-denied: <member> tried to access <resource>")

These render in the standard /audit feed (use the existing audit entry component).

### 5. Backend capability gate (already exists)

Sanity-check the backend capability gate is wired:
- Members with cohort scope should see filtered worker lists (gateway filters `/api/v1/workers` by their cohort scope).
- Workflows + audit entries filtered the same way.
- If backend filtering is missing, file a follow-up gateway brief; web side renders the UX regardless.

### 6. Visual scope-denied banner

When a user navigates to a resource (worker / workflow / audit entry) outside their cohort scope:
- Render a friendly empty-state: "This <worker | workflow | audit entry> isn't in a cohort you have access to. Ask your admin if you need access."
- Soft 403, not a hard error page.

## Acceptance criteria

- [ ] /settings/members row shows cohort-scope chips (or "All cohorts").
- [ ] Edit Member modal includes cohort-scope toggle + multi-select.
- [ ] Save persists to gateway and refreshes the row in place.
- [ ] Invite-member modal includes cohort-scope picker.
- [ ] Cohort-scope-denied audit entries surface in /audit with clear text.
- [ ] Member-facing "not in your cohort" empty state renders cleanly when they hit a scoped-out resource.
- [ ] Owner/admin role overrides cohort scope (those roles always see everything).
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Per-resource (worker-level, workflow-level) sharing overrides — v1.x.
- "Request access" flow from the scope-denied empty state — v1.x.
- Bulk-edit cohort scope across multiple members — v1.x.
- Backend cohort-filtering implementation if missing (separate gateway brief).
