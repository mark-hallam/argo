# codex-mcp-rate-limit-rewrite — per-tenant MCP connector rate limit, rewritten against current API (P1 audit-finding-8)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — architectural call on injection point.
**Depends on**: `mission-tenant-substrate-tables` MERGED (#1777).
**Authored against**: origin/main `6b432ec4` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-mcp-rate-limit-rewrite`.
**Source**: 2026-05-17 audit finding #8 (lines 162-175). Supersedes the prior BLOCKED brief `codex-mcp-connector-rate-limit-per-tenant` which targeted a stale `McpClientPool::dispatch(tenant, connector, args)` API that doesn't exist.

## Mission alignment

- **Multi-tenant by construction** (CLAUDE.md core invariant #2) — per-tenant fairness on outbound MCP calls.
- **Reliability** (#1) — one noisy tenant must not exhaust another's API quota share.
- **Security** (#4) — DoS-by-noisy-neighbor protection.

## Scope

P1. Inject per-(tenant, connector) rate limit at the actual `McpClientPool::acquire` hot path — NOT at a `dispatch` boundary that doesn't exist.

### Architecture (rewritten against real API)

The real `argo-mcp` surface per `crates/argo-mcp/src/pool.rs`:

```rust
pub struct McpClientPool { ... }
impl McpClientPool {
    pub fn new(config: PoolConfig) -> Arc<Self>;
    pub async fn acquire<F, Fut>(&self, key: PoolKey, factory: F) -> Result<PoolGuard, McpError>
        where F: FnOnce() -> Fut, Fut: Future<Output = ...>;
    pub async fn evict_idle(&self);
}
```

`PoolKey` already exists — it's `{ tenant, connector }`-shaped (or extend if not). The rate limit gate fires INSIDE `acquire` before the factory is invoked.

### Injection point

```rust
// crates/argo-mcp/src/pool.rs — extend acquire signature OR introduce
//   acquire_with_rate_limit that the gateway-level callers prefer.

pub async fn acquire(...) -> Result<PoolGuard, McpError> {
    // NEW: consult per-(tenant, connector) bucket BEFORE spinning up subprocess
    let bucket = self.rate_limiter.bucket_for(&key)?;
    if !bucket.try_acquire() {
        let retry_after_ms = bucket.next_token_ms();
        return Err(McpError::RateLimited { retry_after_ms, limit: bucket.sustained });
    }
    // ... existing acquire logic ...
}
```

### Bucket configuration

Reuse `tenant_rate_limit` table (already exists; just add `connector` column):

```sql
-- migration 113 (after substrate-tables; check current next free slot)
ALTER TABLE tenant_rate_limit ADD COLUMN connector TEXT NOT NULL DEFAULT '*';
CREATE INDEX idx_tenant_rate_limit_connector
  ON tenant_rate_limit(tenant_id, connector);
```

Defaults (in-code, override-able via row):

| Connector | Sustained (per-min) | Burst |
|---|---:|---:|
| gmail, slack | 30 | 60 |
| hubspot, salesforce | 60 | 120 |
| stripe | 10 | 20 |
| `*` (default) | 60 | 120 |

### Rate limiter implementation

- Token bucket per `(TenantId, &str)` keyed map.
- `BucketRegistry` uses `lru::LruCache` (already in workspace) capped at 10k pairs.
- Refill task: single `tokio::spawn` at 1Hz that ticks all active buckets. O(N) per tick; bounded by LRU.
- Buckets created lazily on first `acquire` for that (tenant, connector) pair.

### Error variant

Add `McpError::RateLimited { retry_after_ms: u64, limit: u32 }` to `crates/argo-mcp/src/error.rs`. (Per the audit clarifying McpError lives in argo-mcp, not argo-types.)

LOC: ~400.

## Files you may touch

- `crates/argo-store/migrations/113__tenant_rate_limit_connector.sql` (NEW; verify next-free slot before committing). ≤15 LOC.
- `crates/argo-store/src/tenant_rate_limit.rs` (existing — note: `tenant_rate_limit.rs` not `tenant_rate_limit_query.rs` per audit). Extend with connector filter. ≤30 modified LOC.
- `crates/argo-mcp/src/rate_limit.rs` (NEW) — TokenBucket + BucketRegistry + refill task. ≤180 LOC.
- `crates/argo-mcp/src/rate_limit_tests.rs` (NEW) — under/at/over budget, refill, eviction, cross-tenant isolation. ≤120 LOC.
- `crates/argo-mcp/src/pool.rs` — wire bucket consult into `acquire`. ≤30 modified LOC.
- `crates/argo-mcp/src/error.rs` — add `RateLimited` variant. ≤8 modified LOC.
- `crates/argo-mcp/src/lib.rs` — re-export. ≤4 modified LOC.
- `crates/argo-mcp/Cargo.toml` — ensure `lru` (likely already present). ≤2 modified LOC.

## Files you must NOT touch

- Individual MCP connector crates (`argo-mcp-connector-*`) — gate is at pool layer.
- The audit chain — rate-limit rejections are caller-side; audit emissions stay where they are.
- `mission-tenant-substrate-tables` schemas — read-only.
- Pricing-tier enforcement (`claude-pricing-tier-enforcement-runtime` — TODO; runs at request layer, not inside MCP pool).

## Acceptance criteria

- [ ] Migration 113 applies cleanly on fresh + populated DBs (additive).
- [ ] First 30 `acquire(gmail, tenant=A)` calls in 1 minute succeed; 31st returns `Err(McpError::RateLimited { retry_after_ms, limit: 30 })`.
- [ ] Tenant B's quota independent: B can make 30 gmail calls while A is exhausted.
- [ ] Refill task ticks at 1Hz, adding 0.5 tokens per second to each bucket (30/min sustained).
- [ ] LRU eviction at 10k active pairs.
- [ ] `tenant_rate_limit` row override `(tenant=A, connector=gmail, sustained=100)` raises A's gmail cap.
- [ ] `cargo test -p argo-mcp -p argo-store` passes.
- [ ] `cargo clippy -p argo-mcp -p argo-store --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated (this brief; also the prior BLOCKED brief gets marked SUPERSEDED).

## Branch + PR

- Branch: `codex-mcp-rate-limit-rewrite`
- Title: `feat(mcp): per-(tenant, connector) token-bucket rate limit at acquire (codex-mcp-rate-limit-rewrite)`
- Worker verification: crate-scoped + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Bucket key is `(TenantId, &str)`** — connector name. Don't pre-hash; let LRU handle.
- **Don't pre-allocate buckets** — lazy-create on first acquire.
- **`retry_after_ms` computation:** `1000 / refill_rate_per_sec`. For 30/min sustained → 2000ms.
- **No cross-tenant fairness inside the bucket.** Each tenant's bucket is independent.
- **Don't change `acquire`'s F+Fut signature.** Add the rate-limit check inside; the factory only runs if the bucket allows.
- **`McpError::RateLimited` is the new variant** — verify the canonical-error-type lint allows it (per F-08 type canonicalization).
- **Migration slot pick** — verify next free at claim time via `ls crates/argo-store/migrations/ | sort -t_ -k1 -n | tail -1` (last MERGED brief used slot 112 for billing-DLQ).

## Closes audit finding #8

When this merges, the 2026-05-17 audit's finding #8 is resolved. Mark the prior BLOCKED brief `codex-mcp-connector-rate-limit-per-tenant` as SUPERSEDED → DROPPED in the same commit.
