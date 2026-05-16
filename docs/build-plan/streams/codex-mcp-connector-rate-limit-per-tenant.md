# codex-mcp-connector-rate-limit-per-tenant — outbound MCP rate-limit by tenant (P1 architectural)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — architectural decision required (where in dispatch path to gate, error semantics, retry strategy). **Depends on**: `mission-tenant-substrate-tables` MERGED (#1777).
**Authored against**: origin/main `bf84d768` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-mcp-connector-rate-limit-per-tenant`.
**Source**: SWOT enterprise-procurement gap — today an MCP connector (gmail, slack, hubspot, …) is rate-limited at the connector level by the upstream API. A noisy tenant can exhaust ANOTHER tenant's API quota share. Enterprise customers won't accept this.

## Mission alignment

- **Multi-tenant by construction** (CLAUDE.md core invariant #2) — per-tenant fairness on outbound calls.
- **Reliability** (#1) — one noisy tenant must not degrade another's experience.
- **Security** (#4) — DoS-by-noisy-neighbor protection.

## Scope

P1 architectural. Wire `tenant_rate_limit` (existing table) to the outbound MCP-call hot path:

1. **Per-(tenant, connector) token bucket.** Each tenant has a quota per connector, refilled at a configurable rate. Defaults:
   - `gmail`, `slack`: 30 calls/min sustained, 60 burst.
   - `hubspot`, `salesforce`: 60 calls/min sustained, 120 burst.
   - `stripe`: 10 calls/min sustained, 20 burst (Stripe charges per call).
   - `default`: 60/min sustained, 120 burst.
   - Per-tenant overrides via `tenant_rate_limit` table (already exists; add `connector` column).

2. **Bucket lookup hot path.** `McpClientPool::dispatch(tenant, connector, args)` consults the bucket BEFORE spawning the subprocess. Bucket lookup is a in-memory LRU per `(tenant, connector)` keyed map, ~10μs.

3. **Rejection behavior.** Over-budget → return `Err(McpError::RateLimited { retry_after_ms: u64, limit: u32 })` immediately. Caller (typically the worker dispatch loop) emits an audit row `ApprovalAsked` (escalate to operator) OR retries-with-backoff (if the brief's policy is `retry_until_ok`).

4. **Refill background task.** A single tokio task ticks all bucket refills at 1Hz. O(N) per tick where N = active tenant × connector pairs; bounded by LRU eviction (max 10k pairs in memory).

5. **Migration 109** — add `connector TEXT` column to `tenant_rate_limit`; existing rows are unchanged (NULL = applies to all connectors).

LOC: ~400.

## Files you may touch

- `crates/argo-store/migrations/109__tenant_rate_limit_connector.sql` (NEW) — additive ALTER. ≤15 LOC.
- `crates/argo-mcp/src/rate_limit.rs` (NEW) — `TenantConnectorBucket` + `BucketRegistry` + refill task. ≤180 LOC.
- `crates/argo-mcp/src/rate_limit_tests.rs` (NEW) — under/at/over budget, refill, eviction. ≤120 LOC.
- `crates/argo-mcp/src/pool.rs` — wire bucket consult into `McpClientPool::dispatch`. ≤30 modified LOC.
- `crates/argo-mcp/src/lib.rs` — re-export. ≤4 modified LOC.
- `crates/argo-mcp/Cargo.toml` — narrowly: add `lru` if not present. ≤3 modified LOC.
- `crates/argo-store/src/tenant_rate_limit_query.rs` — extend with `connector` filter. ≤30 modified LOC.
- `crates/argo-types/src/error.rs` — add `McpError::RateLimited` variant. ≤8 modified LOC (audit canonical-error-type lint).

## Files you must NOT touch

- Individual MCP connector crates (`argo-mcp-connector-*`) — gate is at the pool layer, not per-connector.
- The audit chain — rate-limit rejections are caller-side; audit rows are emitted by the caller's existing path.
- Workspace-tier-based rate limit from `claude-pricing-tier-enforcement-runtime` (in queue) — that brief gates by tier; this one gates by tenant-connector burst. Both run; tier acts as upper bound, connector bucket prevents in-tier noisy-neighbor.

## Acceptance criteria

- [ ] Migration 109 applies cleanly on a fresh DB and on a populated DB (additive).
- [ ] First 30 calls to `gmail` from tenant A succeed; 31st (within 1 min) returns `Err(McpError::RateLimited { retry_after_ms, limit: 30 })`.
- [ ] Tenant B's quota is independent — B can make 30 calls while A is exhausted.
- [ ] Refill at 1Hz adds 0.5 tokens (30/60s) per tick to each bucket.
- [ ] LRU evicts oldest-touched bucket when active pairs exceed 10k.
- [ ] `tenant_rate_limit` row override: setting `(tenant=A, connector=gmail, sustained=100)` lifts A's gmail cap to 100.
- [ ] `cargo test -p argo-mcp -p argo-store` passes.
- [ ] `cargo clippy -p argo-mcp -p argo-store -p argo-types --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-mcp-connector-rate-limit-per-tenant`
- Title: `feat(mcp): per-tenant per-connector token-bucket rate limit (codex-mcp-connector-rate-limit-per-tenant)`
- Worker verification: crate-scoped + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Bucket key is `(TenantId, &str)`** where the `&str` is the connector name (e.g. `"gmail"`). Don't pre-hash; let LRU handle.
- **Refill task is one tokio::spawn at boot.** Not a per-bucket task — that'd blow up active-task count.
- **LRU eviction on warm bucket = full quota refund.** Conservative: a tenant that comes back after eviction gets a full burst, not a partial. Document in module rustdoc.
- **`retry_after_ms` computation:** time until enough tokens accumulate for 1 call = `1000 / refill_rate_per_sec`. For `sustained=30`/min: 2000ms.
- **No fairness across tenants.** Bucket-per-tenant means each tenant's quota is independent; one over-budget tenant doesn't help/hurt others. Globally, an upstream connector (gmail) is still bounded by the sum of all tenants' caps × refill rate.
- **Cross-cutting with #1759 tier limits.** Pricing-tier-enforcement (in TODO queue) gates by tier. This brief gates by tenant-connector pair. Both can fire — caller hits whichever cap is lower. Document this interaction in `pool.rs`'s top-of-file comment.
- **Don't pre-allocate buckets for all known tenants.** Lazy-create on first call. Saves memory at scale.
