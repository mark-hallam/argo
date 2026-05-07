# Worker archive + rehydrate (sleep/wake)

**Owner**: claude. **Depends on**: —

## Why

Workers accumulate. After a campaign ends, a customer churns, or a
seasonal duty completes, we need to put a Worker to sleep without
deleting them — preserve audit history, charter, persona — and bring
them back later. This is the difference between a temp tool and a
teammate. Backs mission claim 1 ("persistent identity over time").

## Scope

- New table `worker_archive` storing a frozen JSON snapshot of the
  Worker plus archived_at + archived_by_principal.
- `argo worker archive <id> [--reason <txt>]` snapshots and removes
  from the live `worker` table (audit log + handoff history left
  in place; they reference WorkerId not the row).
- `argo worker rehydrate <id>` restores the snapshot back to `worker`,
  with a fresh `last_active_at` (if that column exists post-presence
  brief) and removes the archive row.
- `argo worker archive list` lists archived workers with archived_at
  + reason.
- Rehydrate fails if a live `worker` row already exists with the same id.

## Approach (concrete)

1. **NEW** migration `crates/argo-store/migrations/31__worker_archive.sql`
   (~25 LOC):
   - `worker_archive(id BLOB PRIMARY KEY, tenant_id BLOB NOT NULL,
     archived_at INTEGER NOT NULL, archived_by_principal TEXT NULL,
     reason TEXT NULL, frozen_json TEXT NOT NULL)`
   - index on `(tenant_id, archived_at DESC)`
2. **NEW** `crates/argo-store/src/worker_archive.rs` (~150 LOC):
   - `pub async fn archive_worker(store, id, principal, reason) -> ArgoResult<()>`
     — read row, serde_json::to_string, write archive, delete from `worker`
     in a transaction
   - `pub async fn rehydrate_worker(store, id) -> ArgoResult<Worker>` —
     read archive, deserialize, write back, delete archive in tx
   - `pub async fn list_archived(store, tenant_id) -> ArgoResult<Vec<ArchiveRow>>`
3. **NEW** `crates/argo-store/src/worker_archive_tests.rs` (~180 LOC) —
   round-trip, double-archive (error), rehydrate-when-live (error),
   list ordering.
4. **NEW** `crates/argo-cli/src/handlers/worker_archive.rs` (~220 LOC) —
   3 subcommand handlers + markdown rendering.
5. **NEW** `crates/argo-cli/src/handlers/worker_archive_tests.rs` (~150 LOC).
6. **Modify** `crates/argo-cli/src/cli.rs` — 1 sub-enum
   `WorkerArchive { Archive { id, reason }, Rehydrate { id }, List }`
   under existing `Worker`. EXHAUSTIVE-MATCH note for handlers/mod.rs.

## Files you may touch

- `crates/argo-store/migrations/31__worker_archive.sql` (NEW)
- `crates/argo-store/src/worker_archive.rs` (NEW, ≤ 220 LOC)
- `crates/argo-store/src/worker_archive_tests.rs` (NEW, ≤ 250 LOC)
- `crates/argo-store/src/lib.rs` (1 pub mod line)
- `crates/argo-cli/src/handlers/worker_archive.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/worker_archive_tests.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod + dispatch arms)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — 1 sub-enum)

## Files you must NOT touch

- `audit_log` migration / writer — archive does not delete audit
- `handoff_routing` / `handoff_auctions` — keep historical pointers intact
- `worker_budget` — archive is metadata-only; spend rows stay
- `crates/argo-identity/` — Worker struct unchanged

## Acceptance criteria

- [ ] `argo worker archive <id> --reason "campaign over"` removes from `worker` and writes `worker_archive`
- [ ] `argo worker archive list` returns the archived worker with reason + archived_at
- [ ] `argo worker rehydrate <id>` restores the worker; `worker_archive` row gone
- [ ] Rehydrate when live worker exists → error, no mutation
- [ ] Archive when no live worker exists → error, no mutation
- [ ] Round-trip preserves persona, role, channels, charter (via serde)
- [ ] All ops are transactional (verify by killing mid-tx in a test)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-archive-rehydrate`
- PR title: `worker: archive + rehydrate flow (claude-worker-archive-rehydrate)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies to all `crates/argo-cli/` files.
- Migration number 31: verify with
  `ls crates/argo-store/migrations/ | sort -V | tail -5` before writing.
- Frozen JSON should be the same `Worker` serde format used elsewhere
  so future schema additions roll forward via `#[serde(default)]`.
- Archive is NOT delete. There is no "purge archive" command in this
  brief — that's a future Phase tied to retention policy.
