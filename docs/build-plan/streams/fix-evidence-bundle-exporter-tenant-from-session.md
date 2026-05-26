# fix-evidence-bundle-exporter-tenant-from-session

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-evidence-bundle-exporter-tenant-from-session`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/scene-6-claim-6-trust-and-approvals.md` F10 [P2].

🟡 **P2 — `EvidenceBundleExporter` is invoked with hardcoded `tenantId="tenant_default"` on both the audit feed page and the audit detail page. Every exported ZIP carries `"tenant_default"` in its envelope regardless of who clicked Export. Cross-tenant integrity hole for the export artifact specifically.**

## Root cause

`web/app/(app)/audit/page.tsx:261-262`:
```tsx
<EvidenceBundleExporter tenantId="tenant_default" tenantLabel="Default tenant" />
```

`web/app/(app)/audit/action/[actionId]/page.tsx:152-153`:
```tsx
<EvidenceBundleExporter tenantId="tenant_default" tenantLabel="Default tenant" />
```

Both pages hardcode the value at the call site. The `useViewerPolicy()` hook (likely at `web/lib/hooks/use-viewer-policy.ts`) returns the real workspace id from the session, but it's never threaded into the exporter.

Function-side concern: the ZIP envelope IS a real downloadable artifact (the camera doesn't open it during the demo per runbook, but the operator could). If two tenants both export, both bundles say `tenant_default` — the audit-trail-of-the-export-itself is broken.

Per CLAUDE.md, claim 6 multi-tenant is LIVE with regression test pinned, but this specific export surface bypasses tenant binding.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F10 [P2] — `EvidenceBundleExporter` hardcoded `tenantId="tenant_default"`

## Fix recipe

1. **Thread session workspace id** into both pages. Use the existing `useViewerPolicy()` hook OR a server-component `getSession()` helper if the page is server-rendered:
   ```tsx
   const session = useViewerPolicy();
   ...
   <EvidenceBundleExporter
     tenantId={session.workspaceId}
     tenantLabel={session.workspaceLabel}
   />
   ```

2. **Defense-in-depth in `EvidenceBundleExporter`** at `web/components/skycot/audit/EvidenceBundleExporter.tsx`:
   - If `tenantId === "tenant_default"` or empty, refuse to render the export button and surface a "Export unavailable in this session" tooltip.
   - Log a console.error so future audits catch the regression early.

3. **Gateway-side**: when the export endpoint is hit, the gateway should ignore the `tenantId` query param and resolve from the session principal — DO NOT trust the client's claim. (The SPA-side fix is for the artifact envelope; the gateway-side fix is the real authorization gate. Both belong here.) Locate the export handler under `crates/argo-cli/src/handlers/gateway/routes_audit_export*.rs` and verify it does `resolve_workspace_tenant_for_principal(...)` instead of trusting the param.

4. **Integration test** `crates/argo-cli/tests/audit_export_tenant_isolation.rs`:
   - Tenant A clicks export with `tenantId=A` — envelope contains tenant A's id.
   - Tenant A clicks export with `tenantId=B` (forged) — envelope contains tenant A's id (gateway ignores the param).
   - Tenant A with no `tenantId` param — envelope contains tenant A's id (auto-resolves).

## Acceptance criteria

- [ ] Both `/audit` + `/audit/action/<id>` pages pass the real session workspace id to `EvidenceBundleExporter`.
- [ ] Exported ZIP envelope carries the real `tenant_id`, never `"tenant_default"`.
- [ ] Gateway export handler resolves tenant from principal, ignores client-supplied `tenantId` param.
- [ ] If the SPA can't resolve a session, export button is hidden with explanatory tooltip.
- [ ] Cross-tenant test passes cold: forged `tenantId` param doesn't leak across tenants.

## May-touch

- `web/app/(app)/audit/page.tsx` (thread session id)
- `web/app/(app)/audit/action/[actionId]/page.tsx` (thread session id)
- `web/components/skycot/audit/EvidenceBundleExporter.tsx` (defense + console.error)
- `web/lib/hooks/use-viewer-policy.ts` (verify it exposes workspaceId; add if missing)
- `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs` (or equivalent)
- `crates/argo-cli/tests/audit_export_tenant_isolation.rs` (NEW)

## Out of scope

- Renaming `tenant_id` → `workspace_id` in the bundle envelope (separate brief)
- The export ZIP format (already locked)
- Multi-workspace switcher (v1.1)

## Effort

~80-130 LOC across 2 SPA files + 1 component + 1 gateway handler audit + 1 integration test. ~2-3 hours.
