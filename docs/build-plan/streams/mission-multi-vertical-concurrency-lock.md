# Stream mission-multi-vertical-concurrency-lock — single-worker-per-vertical safety lock

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `8fe7aeec` on `2026-05-13`. **Drift-fix amendment 2026-05-13** (vs `b2b64554`): brief-audit agent flagged the original cite of `sections_runtime.rs` (does not exist on main) and `ArgoError::ContentionTimeout` (variant does not exist) as STALE_PATH. Updated may-touch + acceptance criteria below to target the real surfaces.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-multi-vertical-concurrency-lock`.

> SOURCE: MISSION_BACKLOG.md Section 8 #238 ("Concurrent worker safety (multi-vertical lock)"). Today nothing prevents two parallel processes (operator's CLI + a forge subprocess + a cron tick) from concurrently bidding on the same `(tenant_id, vertical_id, worker_id)` and stepping on each other's state — duplicate dispatches, race-y memory writes, double-spends against the budget gate. A simple advisory lock per `(tenant, vertical, worker)` triple resolves it.

## Scope

Add a `VerticalConcurrencyLock` that wraps `argo-store::SessionStore` with a per-`(tenant, vertical, worker)` advisory lock. The lock uses a dedicated `vertical_locks` table (one row per holder, `holder_pid + acquired_at + lease_secs`) and a poll-based acquire (1 attempt every 100ms, fail after `acquire_timeout_secs`, default 5).

Callers wrap their critical section:

```rust
let _guard = lock.acquire(tenant, vertical, worker).await?;
// ... bid + dispatch + budget gate + audit ...
// guard drops -> row deleted -> lock released
```

Stale-lock recovery: on acquire, if the existing row's `acquired_at + lease_secs < now`, treat it as abandoned and overwrite. Crashed holders never starve the system longer than one lease.

**Not in scope**: distributed locking across multiple hosts (SQLite-only v1 — Postgres-shaped seam can come later); priority queueing; lock-wait fairness; readers/writers split (this is a single mutex per triple).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `<NEW: crates/argo-orchestrator/src/vertical_lock.rs>` — the lock implementation (≤350 LOC). `VerticalConcurrencyLock` (Send+Sync), public methods `acquire(...) -> ArgoResult<LockGuard>` and `force_release(...)` (operator escape hatch). `LockGuard` `Drop` deletes the row best-effort (uses a `tokio::runtime::Handle::block_on` fallback if no async context; if it can't release, the lease expiry catches it).
- `<NEW: crates/argo-orchestrator/src/vertical_lock_tests.rs>` — unit tests (≤250 LOC) covering: acquire-release-reacquire round trip; second-acquire-while-held blocks; stale lease recovery; per-tenant isolation; per-vertical isolation; per-worker isolation.
- `crates/argo-orchestrator/src/lib.rs` — narrowly scoped: add ONE `pub mod vertical_lock;` line + cfg-test mod line. Re-export `VerticalConcurrencyLock, LockGuard, LockConfig`.
- `<NEW: crates/argo-store/migrations/<next_slot>__vertical_locks.sql>` — table DDL only:
  ```
  CREATE TABLE vertical_locks (
      tenant_id TEXT NOT NULL,
      vertical_id TEXT NOT NULL,
      worker_id TEXT NOT NULL,
      holder_pid INTEGER NOT NULL,
      acquired_at INTEGER NOT NULL,
      lease_secs INTEGER NOT NULL,
      PRIMARY KEY (tenant_id, vertical_id, worker_id)
  ) STRICT, WITHOUT ROWID;
  ```
- `crates/argo-store/src/migrations/mod.rs` (currently 493 LOC — registration target depends on `claude-store-migrations-mod-split` landing first; if not yet merged at claim time, target `crates/argo-store/src/migrations/registry.rs` per that split's pattern). Narrowly scoped: register the new slot. No reordering. **CRITICAL**: read the next-slot value at claim time — slot 64 is contested by #1447/#1450/#1464; pick whatever's free.
- `crates/argo-config/src/sections_context.rs` (at `:278`, where `RuntimeConfig` actually lives — file `sections_runtime.rs` does NOT exist on current main) — narrowly scoped: add ONE `pub struct VerticalLockConfig { pub enabled: bool, pub lease_secs: u64, pub acquire_timeout_secs: u64 }` with defaults `enabled=true, lease_secs=30, acquire_timeout_secs=5`. Add ONE field on `RuntimeConfig`.
- `Cargo.toml` — NO new dependencies.

## Files you must NOT touch

- Any `migrations/v60.rs` or `migrations/64__*.sql` — those are in flight in three other open PRs (#1447, #1450, #1464). Pick a clean slot, do not touch v60.rs.
- The dispatch / auction loop call sites — wiring the lock is a deferred follow-up brief (`mission-vertical-lock-wire-auction`).
- `crates/argo-cli/**` — no CLI surface in v1. The lock is a runtime primitive.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason. Especially: if the only free migration slot collides with another open PR's slot, BLOCK and let the conductor sequence.

## Acceptance criteria

- [ ] `VerticalConcurrencyLock::acquire(tenant, vertical, worker)` returns a `LockGuard` whose `Drop` deletes the lock row.
- [ ] Second acquire while held blocks until either (a) the holder drops the guard, or (b) `acquire_timeout_secs` elapses → returns `Err(ArgoError::Provider(format!("vertical-lock contention timeout on (tenant={}, vertical={}, worker={}) after {}s", ...)))`. (`ArgoError::ContentionTimeout` does NOT exist on current main — use `Provider(String)` with a descriptive message per the existing ArgoError variant patterns. If a future brief promotes this to a typed variant, that's a separate concern.)
- [ ] Per-tenant isolation: two tenants holding the same `(vertical, worker)` get distinct locks.
- [ ] Per-vertical isolation: same tenant + worker, two verticals → distinct locks.
- [ ] Per-worker isolation: same tenant + vertical, two workers → distinct locks.
- [ ] Stale-lease recovery: a row with `acquired_at + lease_secs < now` is treated as abandoned; next acquire succeeds without operator intervention.
- [ ] Migration slot chosen does NOT collide with #1447/#1450/#1464 (verify on claim by `gh pr list --state open --json files`).
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-orchestrator` passes.
- [ ] Every modified file < 500 LOC.
- [ ] Additive migration (CREATE TABLE only; no ALTER on existing tables).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-multi-vertical-concurrency-lock`
- PR title: `orchestrator: per-(tenant,vertical,worker) advisory lock (mission-multi-vertical-concurrency-lock)`

## Notes for the implementer

- The poll interval (100ms) and lease (30s default) are conservative. Sub-100ms acquire is fine for the swarm's tick cadence; tighter loops are an over-fit for v1.
- Use `std::process::id()` for `holder_pid` — it's enough for stale-lock recovery on a single host. Distributed unique-host-id is out of scope.
- `LockGuard::Drop` happens on the runtime's tokio worker; for the rare case where it's called outside a tokio context, fall back to `tokio::runtime::Handle::try_current().ok()`. If neither works, log + skip — the lease will expire.
- Tests should exercise the stale-lease path explicitly: insert a row with `acquired_at = now - 60s, lease_secs = 30`, then assert `acquire` succeeds immediately.
- The `vertical_id` is the bound vertical-template id (e.g. `customer-support`, `recruiter-screener`). It's a stable string from the manifest.
- **No CLI surface in v1.** Wiring this lock into the auction / dispatch loop is its own brief — `mission-vertical-lock-wire-auction` — so the lock can be reviewed in isolation.
- Do NOT change the existing `argo-orchestrator/src/session.rs` (the session_id flow). The lock is orthogonal to session identity.
- Additive migration only. Per CLAUDE.md `docs/skycot/operator/migrations.md`, no breaking schema changes.
