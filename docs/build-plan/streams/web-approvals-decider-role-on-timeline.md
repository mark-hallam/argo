# web-approvals-decider-role-on-timeline

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-approvals-decider-role-on-timeline`.

## Summary

#1956 just landed the role gate on `/api/v1/approvals/<id>/decide` and stamps `decider_role` into the audit-row payload. The `/approvals` timeline currently shows "Approved by Mark" — surface the role: "Approved by Admin Mark" (or "Approver Mark" if it's a non-admin approver-role decision).

Pre-condition: **#1956 MERGED ✅**.

## Acceptance

- [ ] `/approvals` activity timeline shows the role label alongside the decider's name when a decision row carries `decider_role` (admin / user / owner / guest per the canonical RBAC role taxonomy).
- [ ] Backward-compat: rows without `decider_role` (pre-#1956) render with the existing "Approved by Mark" format — no broken UI on legacy data.
- [ ] Plain language: "Admin Mark approved this" — not "Decision: approve by Mark (role: admin)". Operator preference.
- [ ] Locked register: `/approvals` is enterprise-restrained per `project_approvals_page_design_locked.md`. No gradient, no Audiowide. Subtle role-chip styling (muted text).
- [ ] One unit/component test: row with `decider_role: "admin"` renders role; row without renders without.
- [ ] No regression on the page's existing severity tiers, SLA chips, or KPI strip.

## May touch

- `web/app/(app)/approvals/page.tsx` and any owned timeline-row component
- `web/lib/api/approvals.ts` (the type for an approval row may need extending with the new field)
- Component test files

## Must NOT touch

- The server-side role gate (#1956 owns that — settled).
- The `/api/v1/approvals/*` endpoints.
- The approvals list filter or KPI strip.

## Notes

- Pre-flight: `rg 'decider_role' crates/` + `rg 'decider_role' web/` to find the canonical field name in the audit row JSON envelope.
- The role taxonomy from #1956 is `owner / admin / user / guest` — verify via `crates/argo-cli/src/handlers/gateway/role_gate.rs` (the canonical source).
- Sizing: 80–150 LOC. Demo-visible improvement to the procurement story.
