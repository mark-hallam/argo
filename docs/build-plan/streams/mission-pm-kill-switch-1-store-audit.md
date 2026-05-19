# Stream P2-12-1 — kill switch: store table + audit kind

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `6bcb9aeb` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-kill-switch-1-store-audit`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §11 P2 #12. Split slice 1/5 of parent `mission-pm-kill-switch` (BLOCKED 2026-05-19 — incomplete may-touch).

## Scope

Foundation slice for the worker-freeze primitive. Lands ONLY the persistence layer + audit-kind variant. No runner check, no CLI, no gateway, no UI — those land in siblings 2-5.

Two things only:

1. A new `worker_freeze` table + CRUD over it (`freeze`, `unfreeze`, `is_frozen`, `list_frozen`).
2. A new `AuditKind::WorkerFreezeBlocked` variant on the existing enum.

Siblings consume these — but this slice is dependency-clean and can land first.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/117__worker_freeze.sql` (NEW) — `worker_freeze` table with columns: `worker_id TEXT NOT NULL PRIMARY KEY`, `frozen_at TIMESTAMP NOT NULL`, `frozen_by TEXT NOT NULL`, `reason TEXT NOT NULL`, `unfrozen_at TIMESTAMP NULL`, `unfrozen_by TEXT NULL`. (One row per worker; freeze toggles in place.)
- `crates/argo-store/src/migrations/v117.rs` (NEW) — migration registration.
- `crates/argo-store/src/migrations/registry.rs` — register slot 117 in the appropriate sibling registry split (post-#1658).
- `crates/argo-store/src/worker_freeze.rs` (NEW) — `pub struct FreezeRecord` + `pub async fn freeze(...)`, `pub async fn unfreeze(...)`, `pub async fn is_frozen(worker_id) -> Result<Option<FreezeRecord>>`, `pub async fn list_frozen() -> Result<Vec<FreezeRecord>>`.
- `crates/argo-store/src/lib.rs` — `pub mod worker_freeze;` + re-export `pub use worker_freeze::FreezeRecord;`.
- `crates/argo-store/src/worker_freeze_tests.rs` (NEW) — unit tests: freeze→is_frozen returns Some; unfreeze→is_frozen returns None; double-freeze updates `frozen_at`/`reason`; cross-tenant isolation (a freeze on `worker_A@tenant_1` is invisible to `worker_A@tenant_2` if any naming collision is possible).
- `crates/argo-audit/src/event.rs` — add `WorkerFreezeBlocked { worker_id: String, reason: String }` variant to `AuditKind` enum. Update any exhaustive `match` arms (compiler will surface them).
- `crates/argo-audit/src/event_tests.rs` (or sibling) — add one round-trip serde test for the new variant.

## Files you must NOT touch

- `crates/argo-agent-runner/**` — runner check lands in sibling 2.
- `crates/argo-cli/**` — CLI lands in sibling 3.
- Gateway routes — land in sibling 4.
- `web/**` — UI lands in sibling 5.

## Acceptance criteria

- [ ] Migration 117 runs cleanly forward; verified by `cargo test -p argo-store migration_upgrade`.
- [ ] `worker_freeze::freeze(...)` inserts/upserts a row; `is_frozen` returns `Some(FreezeRecord)` when frozen, `None` after `unfreeze`.
- [ ] `list_frozen` returns only currently-frozen workers (i.e. `unfrozen_at IS NULL`).
- [ ] `AuditKind::WorkerFreezeBlocked` serdes round-trip.
- [ ] `bash scripts/check-migration-slots.sh` passes (no slot collision).
- [ ] `cargo clippy -p argo-store -p argo-audit --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-store worker_freeze` passes; `cargo test -p argo-audit` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-kill-switch-1-store-audit`
- PR title: `feat(store+audit): worker_freeze table + WorkerFreezeBlocked audit kind (mission-pm-kill-switch-1-store-audit)`

## Notes for the implementer

- This is the foundation slice. Keep it minimal. Do not add a `freeze` capability declaration here — that lands in sibling 3 (CLI) and 4 (gateway). Do not call `freeze`/`is_frozen` from anywhere — siblings 2/3/4 wire the consumers.
- `frozen_by` is a free-text actor string; serialise the same way `mission-pm-capability-audit-events` (P0 #4) serialises Principal actors. If that brief hasn't merged yet, use `String` and follow the same pattern when it lands.
- Cooperative-only — no preemptive kill / SIGKILL. The brief deliberately limits this to the state primitive.

## Out of scope

- Pre-turn / pre-dispatch checks (sibling 2).
- CLI commands (sibling 3).
- Gateway routes + capability (sibling 4).
- Web UI button (sibling 5).
- Time-bounded auto-unfreeze, tenant-wide freeze, audit-trail summary on profile — separate briefs.
