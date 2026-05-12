# Mission load-bearing wire — Per-action-type throttling

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 8 #254. Some action types (e.g. `send_email`, `post_listing`, `create_invoice`) have natural per-minute rate limits at downstream APIs. Today: workers fire as fast as they can, hit 429s, retry. Add operator-tunable per-action-type throttling at the worker dispatch layer.

## Scope

NEW `argo-runtime::action_throttle` token-bucket per action_type:

1. **Config** `[runtime.action_throttle.<action_type>] tokens_per_sec = N; burst = M` — operator-defined per action_type.
2. **Token bucket** per (tenant_id, action_type) tuple — refilled at the configured rate, drained per dispatch.
3. **Wait-on-empty** — if no token available, the dispatch yields back to scheduler with a typed delay (don't busy-wait). Operator-visible via `argo doctor throttles` listing.
4. **Audit row** when throttle delays > 10s (kind: `runtime.action_throttled`, payload: `{ action_type, queued_for_secs, tenant_id }`).
5. **Bypass** — `[runtime.action_throttle] enabled = false` (default off, backward compat).

This is NOT in scope: per-action-instance rate limiting (per-prospect, per-customer — too granular); priority queuing (FIFO in v1); cross-process coordination (single-host coverage; multi-host needs the zero-trust brief).

## Files you may touch

- `crates/argo-runtime/src/action_throttle.rs` (NEW, ≤ 380 LOC)
- `crates/argo-runtime/src/action_throttle_tests.rs` (NEW, sibling, ≤ 300 LOC) — 7 tests (refill / burst / wait-on-empty / per-tenant-isolation / audit-row-on-long-wait / disabled-bypass / multi-action-type)
- `crates/argo-runtime/src/lib.rs` — narrowly scoped: register module + thread the gate (verify dispatch site)
- `argo-config/src/sections.rs` — `[runtime.action_throttle.<action>]` parse
- `crates/argo-cli/src/handlers/doctor_throttles.rs` (NEW, ≤ 180 LOC) — `argo doctor throttles` lists current bucket levels
- `docs/skycot/RUNBOOK.md` — "Action throttling" subsection

## Files you must NOT touch

- Existing dispatch loops (the throttle is a gate; don't reshape the loop)
- Per-provider rate-limit detection (orthogonal; happens at the HTTP layer)

## Acceptance criteria

- [ ] Token bucket refills at configured rate; allows configured burst
- [ ] Wait-on-empty yields (no busy-wait); resumes when token available
- [ ] Per (tenant_id, action_type) isolation (one slow tenant doesn't starve another)
- [ ] Audit row when wait > 10s; configurable threshold
- [ ] Disabled by default — zero overhead when off
- [ ] `argo doctor throttles` lists current state per bucket
- [ ] 7 unit tests; `cargo clippy -p argo-runtime --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-per-action-type-throttling`
- PR title: `runtime: per-action-type token-bucket throttling (mission-per-action-type-throttling)`

## Notes

- Token bucket > leaky bucket here — operators want burst capacity for activity spikes, not strict averaging.
- Per-tenant isolation is the key correctness property. Document the (tenant_id, action_type) tuple keying explicitly.
- The 10s audit threshold is the operator-visible signal — sustained throttling means tokens-per-sec config is too tight for the workload.
