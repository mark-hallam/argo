# Tenant quota: from observational to hard enforcement

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C.
> `crates/argo-store/src/tenant_quota.rs` (498 LOC) records
> quota usage per tenant and surfaces it via `argo tenants quota`,
> but the runtime callers do not currently fail-stop when a
> tenant exceeds its quota — they log and continue. This brief
> wires hard enforcement at the gateway action boundary.

## Scope

Convert tenant quota from observational to enforcing:

1. Add a `QuotaError::Exceeded { kind, current, limit }` variant
   to the existing error enum in `tenant_quota.rs`.
2. Add `TenantQuota::check_or_reject(tenant_id, kind, delta)`
   that returns `Err(QuotaError::Exceeded)` when `current + delta > limit`,
   `Ok(())` otherwise. Single transaction: read current, compare,
   return — no write. (The existing `record_usage` is the
   write-side; this is the gate.)
3. Wire `check_or_reject` into the gateway's `dispatch_route`
   for the four heavy actions:
   `session.create`, `message.add`, `forge.run`, `skill.execute`.
   Each maps to a kind: `sessions`, `messages`, `forge_runs`,
   `skill_invocations`.
4. The error surfaces to the API caller as HTTP 429 with body
   `{"error": "quota_exceeded", "kind": "...", "current": ..., "limit": ...}`.

**Pre-work**: `tenant_quota.rs` is 498 LOC — adding the new
method will push it over 500. Split first: move the 4 quota
kinds enum + serialization into `tenant_quota_kinds.rs`, leave
the store API in `tenant_quota.rs`. Verify both still compile
in isolation.

## Files you may touch

- `crates/argo-store/src/tenant_quota.rs` — narrowly: add
  `check_or_reject`, the new error variant, and remove the kinds
  enum (moved to new file)
- `crates/argo-store/src/tenant_quota_kinds.rs` (NEW, ~60 LOC) —
  the kinds enum + Display + FromStr
- `crates/argo-store/src/lib.rs` — narrowly: declare new module
- `crates/argo-cli/src/handlers/gateway/dispatch_route.rs` —
  narrowly: 4 `check_or_reject` call sites (one per action)
- `crates/argo-cli/src/handlers/gateway/error_response.rs` (or
  the existing error-mapping module — verify) — narrowly: map
  `QuotaError::Exceeded` to HTTP 429 with the JSON body
- `crates/argo-store/tests/tenant_quota_enforcement_e2e.rs`
  (NEW) — 4 tests: under-quota success path, at-limit reject,
  reject body shape, re-check passes after admin raises limit

## Files you must NOT touch

- The `tenant_quota` migration — schema unchanged
- `argo tenants quota` CLI handler — read-only viewer; it
  continues to work
- Other gateway actions — only the four named above gate today

## Acceptance criteria

- [ ] `tenant_quota.rs` < 500 LOC after split
- [ ] `tenant_quota_kinds.rs` compiles standalone
- [ ] `check_or_reject` returns `Err(Exceeded)` correctly when
      current + delta > limit (off-by-one tests included)
- [ ] All four gateway actions gate correctly
- [ ] HTTP 429 response with documented body shape
- [ ] At-limit rejection does NOT consume quota (no write)
- [ ] 4 new e2e tests pass
- [ ] Existing `argo-store` tests stay green
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tenant-quota-enforcement-stricter`
- PR title: `quota: hard enforcement at gateway boundary (claude-tenant-quota-enforcement-stricter)`

## Notes for the implementer

- The split is the risky part — do it as commit 1 with NO
  behavior change, run all argo-store tests, then commit 2 adds
  the new method, commit 3 wires it into dispatch.
- Do NOT change the existing `record_usage` semantics. It still
  writes after a successful action. The gate is a separate check
  before the action runs.
- 429 is the only correct status code (rate-limit / quota
  exceeded). Do NOT use 403 or 503.
- DO NOT introduce a "soft warn" mode in this brief — that's a
  separate concern; this brief is hard-enforce only.
