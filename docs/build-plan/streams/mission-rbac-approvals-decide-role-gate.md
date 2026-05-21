# mission-rbac-approvals-decide-role-gate

**Authored against**: `origin/main` `72e782df` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-rbac-approvals-decide-role-gate`.

## Summary

Auditor recommendation P2 #7 (2026-05-21 baseline). The `/api/v1/approvals/<id>/decide` gateway endpoint has no role gate. Anyone with a valid JWT (cross-team, intern, observer) can approve/refuse — procurement-grade RBAC story is broken for mission claim 6 (multi-channel approval). #1901 closed the cross-tenant audit half; this brief closes the role half.

## Acceptance

- [ ] `POST /api/v1/approvals/<id>/decide` enforces a role gate: caller must be `owner` / `admin` / `approver` for the tenant. `member` / `observer` returns 403.
- [ ] Role list configurable per tenant via existing `tenant_role` table (or whatever the canonical RBAC source is — verify in the crate before authoring).
- [ ] Negative integration test: a `member`-role token gets 403; an `admin`-role token gets 200; audit row records the decider's role.
- [ ] Audit row gains a `decider_role` field (already-present field is preferred — read `crates/argo-types/src/audit.rs` for current shape; if not present, add it via additive migration following [docs/skycot/operator/migrations.md](docs/skycot/operator/migrations.md)).
- [ ] `lib/api/approvals.ts` client surfaces the 403 as a "You don't have permission to approve" toast, not a silent fallback.
- [ ] No regression on cross-tenant audit gate from #1901.

## May touch

- `crates/argo-gateway/src/approvals.rs` (or wherever `/decide` is handled — verify path)
- `crates/argo-rbac/src/` (or `argo-auth-rbac`)
- `crates/argo-types/src/audit.rs` (only if `decider_role` field needs adding)
- `web/lib/api/approvals.ts` (toast path for 403)
- A new migration slot if `decider_role` is added (next free slot via `bash scripts/check-migration-slots.sh`)
- New integration test file under the gateway crate's `tests/` dir.

## Must NOT touch

- Cross-tenant audit gate from #1901 (extend, don't refactor).
- Other gateway endpoints — this brief is `/decide` only.
- `/api/v1/approvals` listing (no role gate change there).
- The approvals UI styling (`/approvals` register decision already locked).

## Notes

- Auditor evidence: wave-10 §4 procurement risk #1. `/decide` is the highest-leverage 5-LOC fix in the v1.0 RBAC story.
- Before authoring the role-list, grep for existing role enums: `rg 'pub enum.*Role' crates/`. Re-use the canonical type per [feedback_brief_authoring_preflight_rules.md].
- Negative-test pattern: follow #1702 (`web-rbac-cross-tenant-negative-test`).
