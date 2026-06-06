# fix-demo-mint-tenant-isolation

**Owner**: claude.
**Status**: TODO — enqueued 2026-06-06 (conductor) per DEC-040 (operator-authorized; **operator reviews the PR before merge** — tenancy/WS1-adjacent, HIGHER complexity).
**Authored against**: origin/main `5e87b3aa5` on `2026-06-06`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-mint-tenant-isolation`.

## Why (live-reproduced — GO/NO-GO 2026-06-06)
Every demo-mint principal (`human:demo+<ts>@skycot.test`) resolves to the SAME
shared tenant: the demo-mint JWT carries no `tenant_id`, so the WS1 tenant resolver
(`resolve_tenant_for_request`: JWT → header → cwd) falls through to the **cwd default
tenant**. Live-verified: a fresh demo-mint principal sees the same 9 seeded workers.
So "sign in to talk to your own workspace" is NOT isolated — and combined with the
tier cap, a demo-mint user can't hire either (lands in the capped shared tenant).
Evidence: `docs/skycot/MISSION-E2E-GO-NO-GO-2026-06-06.md` §Blocker-2.

## What
Give each demo-mint principal an **isolated tenant**. Two viable shapes (pick the
lower-risk one that fits the WS1 resolver):
1. **Stamp a `tenant_id` into the demo-mint JWT** — derive a stable per-principal
   tenant id at mint (`POST /api/v1/demo/mint-principal`) so `resolve_tenant_for_request`
   routes the demo principal to its own tenant (JWT branch wins over cwd).
2. **Provision-a-fresh-tenant-on-mint** — create the tenant row + a minimal usable
   baseline at mint, and bind the principal to it.

Coordinate with the WS1 keystone (#2676 `resolve_tenant_for_request`) so the demo
path uses the JWT tenant branch, not the cwd fallback. Keep the existing shared
*seeded* showcase tenant intact (it's the public browse view); this is about
demo-mint *sign-in* getting an own tenant.

Grep-before-write: `rg 'resolve_tenant_for_request|mint-principal|demo_principal' crates/`.
Do NOT fork tenant resolution — extend the existing resolver path.

## May-touch
- The demo-mint route (`routes_auth*` / `argo_cli::demo_mode` mint-principal).
- Tenant resolution (`resolve_tenant_for_request`) — the JWT `tenant_id` branch.
- Possibly auto-provision (reuse `sprint-ws1-3-auto-provision-tenant-on-verify`
  pattern if applicable).

## Verify
- Two demo-mint principals resolve to DIFFERENT tenants (isolated rosters).
- A fresh demo-mint tenant has 0 workers → can hire its first teammate (with the
  tier fix, `fix-demo-tenant-enterprise-tier`, or as a fresh free-tier tenant: 0<1).
- Cross-tenant regression: a demo-mint principal cannot read the shared seeded
  tenant's workers. Integration test.

## SIZING
If this is too big for one work-unit (tenant-resolution touches several seams),
split into `-jwt-tenant-stamp` (mint side) + `-resolver-honors-demo-tenant`
(resolver side); claim mint side first.
