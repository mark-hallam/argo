# Mission load-bearing wire — per-tenant gateway rate-limiting

**Owner**: claude. **Depends on**: existing IP-based rate limiter (verified at `crates/argo-cli/tests/gateway_ip_rate_limit_smoke.rs` + `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs`); bearer-auth → tenant resolution at gateway boundary.
**Authored against**: origin/main `b908e563` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-per-tenant-rate-limiting`.

> SOURCE: MISSION_BACKLOG.md Section 1 #21 — "Add per-tenant rate limiting." Today the gateway rate-limits by IP only. Mission promise: a noisy or compromised tenant gets throttled at their tenant_id, not at the shared-NAT IP that may serve multiple legitimate tenants.

## Scope

1. **NEW gateway middleware** `RateLimitTenant`:
   - Sliding-window 60s counter keyed by `tenant_id` (resolved from bearer token).
   - Default 600 req/min per tenant (10/s sustained); soft-cap at 1200/min with HTTP 429 + `Retry-After`.
   - Bypasses unauthenticated routes (`/health`, `/livez`, etc.).

2. **Config**: `[gateway.rate_limit.tenant]` in `argo.toml`:
   ```toml
   [gateway.rate_limit.tenant]
   requests_per_minute = 600
   burst_multiplier = 2.0
   ```
   Missing block → use defaults (600 + 2.0).

3. **Per-tenant override** via NEW `tenant_rate_limit` table:
   ```sql
   CREATE TABLE tenant_rate_limit (
       tenant_id BLOB PRIMARY KEY,
       requests_per_minute INTEGER NOT NULL,
       updated_at INTEGER NOT NULL
   );
   ```
   Operator sets via NEW `argo tenant rate-limit set <tenant> --rpm 1500`. Missing row → use config default.

4. **In-process state**: a `DashMap<TenantId, SlidingWindow>` in gateway app state. Cleanup task evicts stale tenant entries every 5min.

5. **Observability**: `tracing::warn!` per 429 + a `gateway_rate_limit_throttle_count{tenant=...}` counter via `argo-telemetry::record`.

6. **Stack with IP limit**: tenant limit is checked AFTER IP limit. Both must pass.

This is NOT in scope: cluster-wide distributed rate-limiting (single-process is fine for v1); per-endpoint rate-limit policies (single global per-tenant for v1); auto-tuning thresholds based on usage patterns.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__tenant_rate_limit.sql` (NEW) — verify next free slot.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW); bump `EXPECTED_SCHEMA_VERSION`.
- `crates/argo-store/src/tenant_rate_limit.rs` (NEW, ≤ 150 LOC) — store helpers.
- `crates/argo-store/src/tenant_rate_limit_tests.rs` (NEW, sibling) — 4 unit tests.
- `crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs` (NEW, ≤ 250 LOC) — middleware.
- `crates/argo-cli/src/handlers/gateway/rate_limit_tenant_tests.rs` (NEW, sibling) — 6 unit tests.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: register middleware after the IP limit. ≤ 15 LOC delta.
- `crates/argo-cli/src/handlers/tenant_rate_limit.rs` (NEW, ≤ 120 LOC) — `argo tenant rate-limit set/get` CLI handler.
- `crates/argo-cli/src/cli_subcommands_tenant.rs` (verify path) — narrowly scoped: ADD `RateLimit` subcommand.
- `crates/argo-config/src/gateway.rs` (verify path) — narrowly scoped: ADD `[gateway.rate_limit.tenant]` parser.
- `crates/argo-cli/tests/gateway_tenant_rate_limit_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration smoke: tenant exceeds rpm → 429 + Retry-After.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs` — IP limiter stable; reuse only
- `crates/argo-cli/src/handlers/gateway/auth.rs` — bearer → tenant resolution; reuse only
- Any non-gateway/non-store/non-config/non-cli crate

## Acceptance criteria

- [ ] Migration adds `tenant_rate_limit` table; EXPECTED_SCHEMA_VERSION bumped
- [ ] Middleware throttles when tenant exceeds rpm; returns 429 + Retry-After
- [ ] Per-tenant override row takes precedence over config default
- [ ] `argo tenant rate-limit set/get` works
- [ ] Unauthenticated routes bypass the tenant limit
- [ ] `tracing::warn!` + counter emitted on each 429
- [ ] 4+6 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-cli -p argo-config` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-per-tenant-rate-limiting`
- PR title: `gateway: per-tenant rate-limiting middleware + override storage (mission-per-tenant-rate-limiting)`

## Notes for the implementer

- Migration slot: verify against in-flight briefs. The `tenant_rate_limit` table is small + new — no FKs.
- The DashMap eviction sweep: 5min interval is fine; document as tunable.
- For "tenant resolution": bearer token → `tenant_id` is already done by `auth.rs`. Don't reimplement.
- Sliding window: simple 60-bucket-per-minute ring buffer per tenant. No fancy algorithms.
- Default 600/min is a starting point — document operator can tune.
