# web-rbac-jwt-role-claim-and-gateway-enforce — wire JWT role claim end-to-end + gateway per-action enforcement

**Owner**: claude.
**Authored against**: origin/main `546e3072` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-rbac-jwt-role-claim-and-gateway-enforce`.
**Priority**: P0 — closes the single biggest unfixed gap from the 25-audit sweep. The wave-3 RBAC audit found 11 of 18 sensitive actions have no role gate; gateway trusts client-supplied `X-Argo-Role` header (curl bypass). Marketing claims "per-action role permissions" but enforcement is missing. Procurement-grade blocker.
**Sizing**: ~600-1200 LOC across `crates/argo-cli/src/handlers/gateway/`, `web/auth.config.ts`, plus tests. Multi-crate. Architectural.

## Why this brief exists

From the wave-3 RBAC audit report (`docs/skycot/audit/rbac/2026-05-20-wave-3-rbac.md`):

> **Procurement-review verdict: FAIL.** A live demo where the seed
> guest user (Naomi, naomi@auditor.gov) clicks Freeze on any worker
> succeeds. The 4-role model is marketing copy. Foundation
> (argo-capability engine + role defaults) is sound; the wiring
> from bearer token → principal-role → gateway capability check
> does not exist. Estimated ~3 briefs of work to fix, but until
> they land, every enterprise RBAC claim on the marketing site is
> unsubstantiated.

## Goals

1. The bearer JWT minted at magic-link verify carries a `role` claim derived from a server-side membership table (NOT a client-supplied header).
2. The gateway middleware reads the role from the JWT (not from `X-Argo-Role`) and enforces a per-action capability map.
3. The 11 unguarded sensitive actions (hire, pause, resume, archive, approvals decide/undo, worker PATCHes, audit feed/detail, billing) are gated.
4. The 5 client-trusted gate actions (freeze, workflow-edit, hard-delete, support tickets, cross-tenant audit) are migrated to JWT-based gating.
5. Cross-tenant audit (`X-Argo-Audit-Cross-Tenant: 1`) requires a capability check, not just header presence.
6. Negative tests prove: curl with `X-Argo-Role: owner` does NOT bypass gates.

## Steps (rough; the worker decides exact slicing)

1. Add `role` column to the membership/workspace-member table (additive migration; new slot).
2. Update `auth.config.ts` JWT mint path to look up role and include in claims.
3. Add gateway middleware that decodes role from bearer JWT.
4. Define capability-map (per-action → minimum role).
5. Apply gate to each of the 18 audited routes.
6. Delete the client-trusted `X-Argo-Role` and `X-Argo-Audit-Cross-Tenant` header trust paths.
7. Add negative-tests: `curl -H "X-Argo-Role: owner" /api/v1/workers/<id>/freeze` returns 403 when the bearer JWT role is guest.

## may-touch

- `crates/argo-cli/src/handlers/gateway/` (most gate-handler files)
- `crates/argo-capability/src/` (capability-map definition)
- `crates/argo-identity/src/` (membership-row role column + reader)
- `crates/argo-store/src/migrations/` (additive migration for role column)
- `web/auth.config.ts` (JWT role-claim emission)
- `web/lib/hooks/use-viewer-policy.ts` (read role from JWT not header)
- Negative-test files in `crates/argo-cli/src/handlers/gateway/`

## must-NOT-touch

- Existing migration files (additive only)
- The wave-3 over-claim-softened marketing copy in `/landing`/`/pricing`/`/compare` (already says "per-action role permissions" — wait until enforcement ships before strengthening)

## Acceptance criteria

- [ ] Membership table has a `role` column (or equivalent membership-role join).
- [ ] JWT minted at magic-link verify carries `role` claim (validated via `jwt.io` decode in a unit test).
- [ ] Gateway reads role from bearer JWT, not `X-Argo-Role` header.
- [ ] All 18 sensitive actions audited in the wave-3 report are gated against the capability map.
- [ ] Negative tests: curl with bogus `X-Argo-Role: owner` does NOT bypass gates for any of the 18 actions.
- [ ] Cross-tenant audit requires an explicit cross-tenant capability (not just header presence).
- [ ] tsc clean, clippy --workspace clean, all existing tests pass.
- [ ] Wave-3 RBAC audit verdict moves FAIL → PASS on re-run.

## References

- `docs/skycot/audit/rbac/2026-05-20-wave-3-rbac.md` (the audit that motivated this)
- `crates/argo-capability/` (sound foundation engine; wire-up brief)
