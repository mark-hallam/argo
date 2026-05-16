# claude-pricing-tier-enforcement-runtime — wire pricing tier to runtime gates (Wave 2 follow-up P0)

**Owner**: claude. **Depends on**: `claude-pricing-plans-config` MERGED (#1759, 0cefe82).
**Authored against**: origin/main `2b984b94` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-pricing-tier-enforcement-runtime`.
**Source**: PR #1759 follow-up — tier-storage shipped (workspace.tier column + tenant_quota + tenant_usage), but runtime gates do NOT yet consult it. Free-tier customers can do unlimited actions today.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — billing without enforcement is theatre.
- **Reliability** (#1) — tier gates fail-closed; over-quota fails with `429 Too Many Requests` + Retry-After.

## Scope

P0. Wire `workspace.tier` (`'free'` / `'pro'` / `'enterprise'`) to three runtime gates:

1. **Per-tier action quota.**
   - Read tier from workspace row (joined into the request `Principal`).
   - Per-month action cap: free=100, pro=10_000, enterprise=unlimited (`u32::MAX`).
   - Counter increments in `tenant_usage.used_actions` per action; resets at month boundary.
   - Over-quota → `429 Too Many Requests`, response body `{ "error": "quota_exceeded", "tier": "free", "limit": 100, "used": 100, "retry_after_ts": 1234567890 }`.
   - Hook into existing `routes_dispatch_post.rs` action-execution path (single chokepoint per per-F-13 smoke).

2. **Per-tier feature gates.**
   - `free`: no MCP connectors EXCEPT `gmail`, `slack` (lowest-risk integrations).
   - `pro`: all in-core tools + all MCP connectors.
   - `enterprise`: pro + cross-tenant audit + SCIM + custom roles.
   - Gate at `Tool::required_capabilities()` check or at the dispatch layer; reject with `403 Forbidden + tier_upsell_url` if denied.

3. **Per-tier rate limits (rps).**
   - `free`: 1 rps sustained, 5 rps burst.
   - `pro`: 10 rps sustained, 50 rps burst.
   - `enterprise`: 100 rps sustained, 500 rps burst.
   - Token-bucket per (tenant, route-class). Reuses existing `tenant_rate_limit` infra.

LOC: ~400.

## Files you may touch

- `crates/argo-identity/src/tier.rs` (NEW or extend if pricing-plans landed it) — `pub enum Tier { Free, Pro, Enterprise }` + `pub fn limits(self) -> TierLimits`. ≤100 LOC.
- `crates/argo-identity/src/tier_limits.rs` (NEW) — `pub struct TierLimits { monthly_actions, allowed_connectors, rps_sustained, rps_burst }`. ≤60 LOC.
- `crates/argo-identity/src/lib.rs` — re-export. ≤4 modified LOC.
- `crates/argo-store/src/tenant_query.rs` (existing — extended by #1759) — `pub fn get_tenant_tier(conn, tenant_id) -> Result<Tier>`. ≤30 modified LOC.
- `crates/argo-cli/src/handlers/gateway/quota_middleware.rs` (NEW) — axum middleware that reads tier + quota + denies over-quota. ≤120 LOC.
- `crates/argo-cli/src/handlers/gateway/tier_capability_filter.rs` (NEW) — extends `Principal.role.capabilities` based on tier, narrowing MCP-connector allowlist for `free`. ≤80 LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — mount middleware. ≤8 modified LOC.
- `crates/argo-cli/src/handlers/gateway/quota_middleware_tests.rs` (NEW) — 4 cases (under/at/over quota; tier promotion lifts limit). ≤100 LOC.
- `crates/argo-cli/src/handlers/gateway/tier_capability_filter_tests.rs` (NEW). ≤80 LOC.

## Files you must NOT touch

- `argo-identity::roster` — roster is per-team; tier is per-tenant. Different concern.
- The `Capability` enum itself — extend its consumers, not the enum.
- Existing rate-limit middleware — replace its hard-coded values with tier-aware lookups, but don't restructure.
- Anything in `argo-billing` / Stripe — that brief (`claude-stripe-subscription-integration`) WRITES tier; this brief READS it.

## Acceptance criteria

- [ ] `Tier::Free.limits().monthly_actions == 100`.
- [ ] Calling `/api/v1/dispatch/post` 100 times in a calendar month under `free` tier → 100th succeeds, 101st returns 429 with body matching the JSON shape above.
- [ ] Tier upgrade from free→pro lifts the limit instantly (re-read tier per request, no caching beyond 60s LRU).
- [ ] Calendar-month boundary resets the counter (test via mocked clock).
- [ ] Free-tier worker that tries to dispatch via `argo-mcp-connector-hubspot` → 403 with `tier_upsell_url`.
- [ ] Free-tier worker dispatching via `gmail` or `slack` → 200.
- [ ] Rate-limit: 6 requests in 1 second under `free` → 6th returns 429.
- [ ] All existing routes still pass current tests (no regression).
- [ ] `cargo test -p argo-cli -p argo-identity -p argo-store` passes.
- [ ] `cargo clippy -p argo-cli -p argo-identity -p argo-store --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-pricing-tier-enforcement-runtime`
- Title: `feat(identity): wire tier → quota + connector allowlist + rate limit (claude-pricing-tier-enforcement-runtime)`
- Worker verification: crate-scoped tests + clippy + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Tier read is hot-path.** Cache `(tenant_id, tier, expires_at)` in an LRU (60s TTL). Don't hit SQLite per-request.
- **Free-tier connector allowlist is the only place tier intersects capabilities.** Don't conflate with `Capability::ToolDispatch{tool_name_glob}` — tier is a separate filter applied AFTER the role check passes.
- **`u32::MAX` for enterprise is fine** — practically infinite. Don't model "unlimited" as `Option<u32>`.
- **`retry_after_ts` is Unix epoch seconds** of next month boundary (when quota resets). Match standard HTTP semantics.
- **Period boundary is calendar month** — first of the month at 00:00 UTC. Not a rolling 30-day window. Matches Stripe's invoice period.
- **Cross-tier scenarios** — if a tenant is mid-month and upgrades free→pro, the counter doesn't reset; they get pro's higher cap from now. Don't re-imburse the free-tier consumed actions.
