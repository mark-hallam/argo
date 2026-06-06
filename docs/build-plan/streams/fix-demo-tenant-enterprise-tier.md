# fix-demo-tenant-enterprise-tier

**Owner**: claude.
**Status**: TODO — enqueued 2026-06-06 (conductor) per DEC-040 (operator-authorized; **operator reviews the PR before merge** — touches pricing/tenant config).
**Authored against**: origin/main `5e87b3aa5` on `2026-06-06`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-tenant-enterprise-tier`.

## Why (live-reproduced — GO/NO-GO 2026-06-06)
A Genesis hire in the demo/showcase tenant 403s `tier_worker_cap`:
```
{"error":"tier_worker_cap","tier":"free","used":9,"max_workers":1,...}
```
Root cause: `argo-demo-seed` never sets `workspace.tier`, so the showcase tenant
defaults to **free** (`max_workers:1`) while seeded with 9 workers → every new hire
is refused. This makes the live showcase unable to demonstrate hiring (the headline
flow). Evidence: `docs/skycot/MISSION-E2E-GO-NO-GO-2026-06-06.md` §Blocker-1.

## What
In the demo seed, after the demo workspace/tenant is created/ensured, set its
`workspace.tier` to **`enterprise`** (`max_workers:0` = unlimited; tiers defined in
`docs/skycot/pricing/v1-tiers.toml`). Find the write path: `tenant_query::tenant_tier`
reads the `workspace.tier` column; there's no CLI/route to set it (tests use
`UPDATE workspace SET tier=? WHERE tenant_id=? AND name=?`) — add a small store
helper `set_workspace_tier(tenant, tier)` (or reuse `ensure_pricing_schema` +
direct update) and call it from the canonical demo seed.

**Scope guard:** this is the SEEDED showcase tenant ONLY. A real new signup must
stay **free** tier (the intended funnel — hire 1, hit cap, upsell). Do NOT change
the default tier for fresh tenants.

## May-touch
- `crates/argo-demo-seed/src/seed.rs` (or wherever `run_canonical_v1_seed` creates
  the demo workspace).
- `crates/argo-store/src/tenant_query.rs` (or `tenant_quota.rs`) — a `set_workspace_tier`
  helper if one doesn't exist (grep `tenant_tier` first).

## Verify
- Re-seed the demo tenant → `tenant_tier(demo) == "enterprise"` (a demo-seed unit/
  integration test).
- A Genesis hire in the demo tenant succeeds (no `tier_worker_cap` 403).
- A fresh (non-demo) tenant is still `free` (no regression to the funnel).
