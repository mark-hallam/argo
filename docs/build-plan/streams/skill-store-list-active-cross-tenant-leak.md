# skill-store-list-active-cross-tenant-leak

**Owner**: claude or codex — **WS1 TENANT-ISOLATION → conductor adversarial review before merge, do NOT self-merge.**
**Depends on**: none.
**Authored against**: origin/main `de0e01ea3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh skill-store-list-active-cross-tenant-leak

## Problem (conductor-verified by direct read 2026-06-12)

The argo-skill library's two active-skill read methods have **no tenant
filter**, so they return EVERY tenant's active skills:

- `SkillStore::find_active_by_name` (`crates/argo-skill/src/store_lookups.rs:52-66`):
  `... WHERE s.name = ? AND s.lifecycle_state = 'active'` — no `tenant_id`.
- `SkillStore::list_active` (`store_lookups.rs:143-156`):
  `... WHERE s.lifecycle_state = 'active'` — no `tenant_id`.

The `skill` table **does** carry `tenant_id` (migration
`crates/argo-store/migrations/131__skill_tenant_id.sql`), and the argo-store
layer has a pin test (`skill_query_tenant_bind_pin.rs`) — but these
argo-skill library methods were never bound. Consequence: a tenant's skill
lookup / active-skill list can surface another tenant's skills (cross-tenant
read; also a skill-name probing channel via `find_active_by_name`). This is
the same class the #2836 (p1a) review flagged for `find_active_by_name`,
now confirmed to also hit `list_active`.

## Fix (additive tenant param; reuse, no parallel surface)

Add tenant-scoped variants (additive — keep the old signatures delegating
with a deprecation marker if other callers can't be updated in one pass, or
thread `tenant_id` through all callers if the set is small — grep first):
both queries gain `AND s.tenant_id = ?`. Thread the caller's tenant from
the gateway routes that consume these (dispatch / skills_response / show /
quality / refine — enumerate the real callers, ~6-8). Mirror the
`*_for_tenant` additive pattern p1a established in `store_vertical.rs`.

## May touch

- `crates/argo-skill/src/store_lookups.rs` (+ tests)
- the gateway routes that call these two methods (thread tenant)
- `crates/argo-skill/` callers of the two methods

## Acceptance / verifier

1. Both methods filter by `tenant_id`; a cross-tenant negative test
   (tenant B never sees tenant A's active skill via name-lookup OR
   list-active) — would FAIL on current main.
2. Existing skill tests green; `cargo test -p argo-skill` + the argo-store
   `skill_query_tenant_bind_pin` regression green; clippy clean.
3. No new pub types (`rg 'pub (struct|enum|type)'` delta = 0).

~180-220 LOC. **🔴 P1 — WS1 cross-tenant skill read at the library layer.**
