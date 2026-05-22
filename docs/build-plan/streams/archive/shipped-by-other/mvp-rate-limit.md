# mvp-rate-limit — per-tenant token-bucket rate limit at gateway entry

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — closes hardening gap for v1.0.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

Per-route rate limit (#1077) and per-IP rate limit (#1082) exist. Per-tenant rate limit does NOT — and is the dimension that matters for multi-tenant abuse defense (a single tenant DoS-ing their own quota is acceptable; what's NOT acceptable is one tenant burning shared infra). Add a third middleware layer keyed on resolved `principal.tenant_id`.

## Approach (concrete)

1. **Middleware**: NEW `crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs` (≤ 300 LOC). Mirror the structure of `rate_limit_ip.rs`:
   - `TokenBucket` keyed by `tenant_id`
   - Default: 60 req/min/tenant; env override `ARGO_RATE_LIMIT_TENANT_PER_MIN`
   - Returns HTTP 429 with `Retry-After` header on overflow
   - Ordering at gateway entry: bearer-auth resolves principal → tenant rate-limit gate → per-IP gate → per-route gate → dispatch
2. **Wire**: insert in `crates/argo-cli/src/handlers/gateway/routes.rs` after bearer-auth (so we have a tenant_id) and before per-route gate. Bypass when `ARGO_GATEWAY_AUTH_BYPASS=1` (no principal in test mode).
3. **Tests** (`rate_limit_tenant_tests.rs`, ≤ 200 LOC): bucket fill/drain, refill, two-tenant isolation (tenant A's bucket doesn't drain tenant B's), env override, 429 response shape.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/gateway/rate_limit_tenant_tests.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` declaration
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: 1 middleware call site

## Files you must NOT touch

- `rate_limit.rs` (per-route) / `rate_limit_ip.rs` (per-IP) — leave alone
- Any other gateway handler beyond routes.rs

## Acceptance criteria

- [ ] New module ≤ 300 LOC; tests file ≤ 200 LOC
- [ ] Two-tenant isolation test passes (no cross-bucket drain)
- [ ] Env override works
- [ ] 429 response has correct `Retry-After`
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli rate_limit_tenant` 100% pass

## Branch + PR

- Branch: `mvp-rate-limit`
- PR title: `gateway: per-tenant token-bucket rate limit (mvp-rate-limit)`
