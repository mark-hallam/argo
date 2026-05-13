# Stream mission-prompt-cache-session-affinity — sticky-session affinity for warm prompt cache

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `8fe7aeec` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-prompt-cache-session-affinity`.

> SOURCE: MISSION_BACKLOG.md Section 8 #245 ("Sticky-session affinity (worker reuses same session warm cache)"). Distinct from already-shipped `mission-prompt-cache-hit-rate-observability` (#1300, MERGED) which only reports hit rate. This brief raises the hit rate.

## Scope

A worker that picks up a turn picks a fresh provider session id every time today. The Anthropic `cache_control` markers we send on the system prompt only get a cache hit when the same session_id replays inside the 5-minute TTL window. Today the runtime keys session ids by turn, so worker 1's turn 5 cannot reuse the warm cache from worker 1's turn 4 — every cache row is created and immediately abandoned.

Wire a `PromptCacheAffinity` layer on top of the existing session-id flow. When the runtime begins a new turn for `(tenant_id, worker_id)`, it asks the affinity layer for the **most-recent session id used by that pair within the last 5 minutes**. If one exists, reuse it; otherwise mint a new one and record it.

The cache-read counter from `argo-telemetry::metrics_prompt_cache` (already shipping) should rise after this lands. Use the integration test from `crates/argo-cli/src/handlers/provider_cache_stats.rs` as the verification fixture.

**Not in scope**: cross-worker session sharing (tenants reusing each other's cache); cross-tenant affinity; persistent affinity that survives process restart; affinity beyond Anthropic provider.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `<NEW: crates/argo-runtime/src/prompt_cache_affinity.rs>` — the registry (≤350 LOC). Holds an in-memory `HashMap<(TenantId, WorkerId), (SessionId, Instant)>` behind a `Mutex`. Two pure operations: `acquire_or_mint(tenant, worker, ttl) -> SessionId` and `record(tenant, worker, session)`.
- `<NEW: crates/argo-runtime/src/prompt_cache_affinity_tests.rs>` — unit tests (≤200 LOC).
- `crates/argo-runtime/src/lib.rs` — narrowly scoped: add ONE `pub mod prompt_cache_affinity;` line + the `#[cfg(test)] mod prompt_cache_affinity_tests;` line. No other edits.
- `crates/argo-runtime/src/planner.rs` — narrowly scoped: at the one site that mints a fresh `SessionId` per turn, call `prompt_cache_affinity::acquire_or_mint(...)` and on turn-end call `record(...)`. ≤15 modified lines.
- `crates/argo-config/src/sections_runtime.rs` — narrowly scoped: add ONE `pub struct PromptCacheAffinityConfig { pub enabled: bool, pub ttl_secs: u64 }` with default `enabled=true, ttl_secs=270` (one Anthropic cache TTL minus 30s headroom). Add ONE field on the runtime root.
- `Cargo.toml` (workspace) — NO new dependencies. Use `std::time::Instant`.

## Files you must NOT touch

- `crates/argo-provider/**` — provider doesn't need to know about affinity; it just sees a session_id and inserts `cache_control` like today.
- `crates/argo-telemetry/**` — the counter wiring is already shipped; this brief only causes the counter to climb.
- Any `mission-prompt-cache-hit-rate-observability` artifacts.
- `crates/argo-store/**` — affinity is in-memory only in v1 (persistence is a deferred follow-up).

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `crates/argo-runtime/src/prompt_cache_affinity.rs` defines a public `PromptCacheAffinity` (Send+Sync) with `acquire_or_mint` and `record`.
- [ ] `acquire_or_mint` returns the recorded session id when called within `ttl_secs`; mints fresh after.
- [ ] Per-tenant isolation: `(tenant_a, worker_x)` never gets `(tenant_b, worker_x)`'s session id, even at identical wall-clocks.
- [ ] `planner.rs` integration: turn-start uses affinity; turn-end records.
- [ ] Disabled-config path: `enabled=false` returns a fresh SessionId every call (pre-brief behavior).
- [ ] Integration test in `prompt_cache_affinity_tests.rs` verifies a 4-turn replay sees a single recorded session id when `ttl_secs >= turn_interval`.
- [ ] `cargo clippy -p argo-runtime --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-runtime` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-prompt-cache-session-affinity`
- PR title: `runtime: sticky-session affinity for prompt cache (mission-prompt-cache-session-affinity)`

## Notes for the implementer

- The 5-minute Anthropic cache TTL is the load-bearing constant. Default `ttl_secs=270` keeps a 30s headroom for clock skew + network jitter. Operators can raise on dedicated infra.
- A worker that goes idle for >5 min and then comes back will miss the cache — that's correct, no special handling.
- `Mutex<HashMap>` is fine; if profiling shows contention we'll swap to `DashMap` in a follow-up. Don't pull in `parking_lot` or `dashmap` for v1.
- Per-tenant isolation is the **only correctness invariant** here. Tests must include the cross-tenant collision case.
- The new struct on `RuntimeConfig` must default to enabled — operators should not need to opt in to a strict cache win.
- Do not touch the audit pipeline; affinity is a perf optimization, not a security boundary.
