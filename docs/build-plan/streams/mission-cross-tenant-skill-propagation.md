# Mission load-bearing wire — Cross-tenant skill propagation

**Owner**: claude. **Depends on**: `mission-enterprise-cross-tenant-audit` MERGED (tenant_id threading) + skill-publish-verified-ingest MERGED.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 6 #204. Today skills propagate within a tenant. The next leverage: when a skill reaches `PublishedVerified` trust grade in tenant A, surface it to tenant B's marketplace as a candidate (NOT auto-installed). Operators in B decide to opt-in.

## Scope

NEW marketplace surface for cross-tenant skill discovery:

1. **Backend** `GET /api/v1/marketplace/cross-tenant-candidates` returns published-verified skills from OTHER tenants in the same operator-defined `skill_marketplace_pool` (operators opt their tenant into a pool; default: none, so back-compat).
2. **Per-skill `installation_consent` workflow** — opting tenant clicks Install, gets a multi-step consent: (a) skill source attribution (origin tenant, attestation hash), (b) what tools it requires, (c) what memory it reads/writes. Approval-gated.
3. **Audit row** per cross-tenant install (kind: `marketplace.cross_tenant_install`, payload: full attestation chain).
4. **Frontend** `/marketplace/cross-tenant` route showing the candidate list + installation flow.
5. **Backend** `POST /api/v1/marketplace/cross-tenant-candidates/<id>/install` performs the install (verified-signature check + dependency-graph check + tool-permission check).

This is NOT in scope: tenant-discovery (operators manually configure pool membership); reputation system (next-level brief); auto-install rules; revenue sharing.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_marketplace_cross_tenant.rs` (NEW, ≤ 400 LOC) + tests
- `crates/argo-store/src/marketplace_pool.rs` (NEW, ≤ 280 LOC) — pool membership + candidate discovery
- `crates/argo-store/migrations/73__marketplace_pool.sql` (NEW, ≤ 60 LOC) — `marketplace_pool` table + `pool_member` join
- `web/app/(app)/marketplace/cross-tenant/page.tsx` (NEW, ≤ 320 LOC)
- `web/components/skycot/CrossTenantSkillCard.tsx` (NEW, ≤ 280 LOC)
- `web/components/skycot/InstallConsentDialog.tsx` (NEW, ≤ 320 LOC)
- `web/lib/api/marketplace-cross-tenant.ts` + hook (NEW, ≤ 200 LOC each)
- `argo-config/src/sections.rs` — `[marketplace.cross_tenant]` section
- `docs/skycot/RUNBOOK.md` — "Cross-tenant marketplace" subsection

## Files you must NOT touch

- The skill-verification logic (the verified-signature check uses existing primitives)
- The within-tenant skill propagation (orthogonal)

## Acceptance criteria

- [ ] Migration 73 applies cleanly
- [ ] Discovery endpoint scoped by marketplace_pool membership
- [ ] Install consent flow shows: source attribution + tools required + memory access
- [ ] Verified-signature check enforced (refuse install if signature doesn't validate against origin tenant's key)
- [ ] Audit row per install with full attestation chain
- [ ] Default: tenant in no pool = no cross-tenant marketplace shown (back-compat)
- [ ] 5 backend tests + 6 frontend tests; clippy + pnpm test clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-cross-tenant-skill-propagation`
- PR title: `marketplace: cross-tenant skill discovery + install consent (mission-cross-tenant-skill-propagation)`

## Notes

- The opt-in posture is critical — operators get the value of cross-tenant skill sharing without paying for a noisy global marketplace. Pool membership is the operator's lever.
- Consent dialog is more than UX — it's the legal-evidence-trail for what the operator agreed to when installing. Audit-row captures it fully.
- Migration slot 73 follows 71 (approval-bundle) + 72 (retros).
