# mvp-gateway-request-audit — append one audit-chain entry per HTTP request

**Owner**: claude. **Depends on**: nothing. **Tier P (production-readiness, MVP).**

**Authored against**: origin/main 5baf15a6 on 2026-05-10.

## Why

MISSION_BACKLOG item #22: "Add request audit log entry per HTTP
request." We already have a tamper-evident audit chain
(`crates/argo-audit/`) and bearer-auth middleware that resolves a
`Principal`, but HTTP requests themselves are not logged into that
chain. For a v1 production gateway every authenticated mutation
should be auditable — that's the whole point of having a Principal
plumbed through.

## Approach (concrete)

1. **Add a thin audit wrapper** at the top of the gateway dispatch
   path. The dispatch entry lives in
   `crates/argo-cli/src/handlers/gateway/routes.rs` (currently 129 LOC,
   wide headroom). After auth resolves and BEFORE the route runs,
   capture: `method`, `path` (no query string — query may contain
   secrets), `tenant_id`, `principal_id`, `started_at`. After the
   route returns, capture: `status_code`, `duration_ms`. Emit one
   `AuditEvent` of new variant `AuditKind::HttpRequest` (or
   reuse an existing variant if there's a clean fit — read
   `crates/argo-audit/src/event.rs` first to decide).
2. **If the route returns 401/403/5xx**, still log the entry — that's
   half the value of having request audits.
3. **Skip audit for `/health*` and `/metrics*`** to avoid filling the
   chain with liveness probe noise. Make the skip list a single
   `const SKIP_PATHS: &[&str]` so future endpoints can opt in/out
   trivially.
4. **3 unit tests** (in a sibling `routes_audit.rs` or an inline
   `#[cfg(test)] mod tests` — choose to stay under 500 LOC):
   - successful 200 emits an audit entry with the expected fields
   - 5xx still emits an audit entry with status=5xx
   - `/health` does NOT emit (skip-list works)
5. **1 integration test** in `crates/argo-cli/tests/` that spins up
   the gateway, makes an authenticated request, and asserts a row
   appeared in the audit chain via `verify_all` or direct store query.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes.rs` (wrapper at dispatch)
- `crates/argo-cli/src/handlers/gateway/routes_audit.rs` (NEW sibling,
  ≤ 200 LOC, holds the wrapper helper + skip-list + unit tests)
- `crates/argo-audit/src/event.rs` ONLY if adding a new
  `AuditKind::HttpRequest` variant (one enum arm + 1 line in any
  exhaustive match outside this crate is allowed under the
  EXHAUSTIVE-MATCH amendment).
- `crates/argo-cli/tests/gateway_request_audit.rs` (NEW integration
  test, ≤ 150 LOC)

## Files you must NOT touch

- Any other gateway route file. The wrapper goes ONE layer above
  dispatch; individual route handlers stay untouched.
- `crates/argo-audit/src/sink.rs` / `verify.rs` — no chain protocol
  changes. Use the existing append API only.

## Acceptance criteria

- [ ] Each non-skip HTTP request appends exactly one entry to the
      audit chain.
- [ ] Skip-list (`/health*`, `/metrics*`) prevents probe noise.
- [ ] Failure responses (401/403/5xx) still produce an audit entry.
- [ ] Audit chain `verify_all` still passes after a request burst
      (no chain-break regressions).
- [ ] `cargo clippy -p argo-cli -p argo-audit --all-targets -- -D
      warnings` clean.
- [ ] `cargo test -p argo-cli` green; new integration test passes.
- [ ] No file crosses 500 LOC.

## Branch + PR

- Branch: `claude-mvp-gateway-request-audit`
- PR title: `gateway: per-request audit-chain entry with principal + latency (mvp-gateway-request-audit)`
