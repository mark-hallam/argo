# Mission load-bearing wire — IdentityStore::delete_worker (cascade after archive grace)

**Owner**: claude. **Depends on**: `argo-store::worker_archive::archive_worker` SHIPPED (verified at `crates/argo-store/src/worker_archive.rs:73`); `rehydrate_worker` SHIPPED (line 165); `list_archived` SHIPPED (line 216).
**Authored against**: origin/main `b66b1924` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-identity-delete-worker`.

**DEEP-DRIFT — ALREADY SHIPPED**: per ledger note 2026-05-13, this brief shipped via PR #1290 (commit f6c2ae34) merged 2026-05-11. `worker_archive.rs::delete_worker` + `force_delete_worker` + `delete_grace_days` + migration 51 + `worker_archive_delete_tests.rs` all present on origin/main. Conductor should convert ledger row to MERGED (#1290) or delete; brief was re-added in error during 2026-05-13 replenishment. **No worker action — wait for conductor to flip ledger.**

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 1 item #40 ("Add `IdentityStore::archive_worker` / `delete_worker`"). Archive is shipped; **delete is the gap**. Operators today can archive a worker (soft-delete with rehydration option) and list archived rows, but cannot permanently purge — meaning archived rows accumulate forever, fail GDPR cascade alignment, and clutter `list_archived` indefinitely.

## Scope

Add `delete_worker(worker_id, tenant: &TenantId) -> ArgoResult<()>` that **permanently removes** an archived worker row plus any downstream rows that explicitly cascade (memory rows tagged with this worker_id, skill quality scores keyed to this worker_id, audit-row references where retention policy permits delete).

**Guard rails (all enforced before any DELETE):**
1. Row MUST be in `worker_archive` table (NOT in live `worker` table) — refuse with `ArgoError::Schema` otherwise.
2. Archive `archived_at` MUST be older than `grace_window` (default 30 days; configurable via `ARGO_WORKER_DELETE_GRACE_DAYS`). Refuse with explicit error citing the remaining window.
3. Cascade is scoped to **per-tenant** — never touch cross-tenant rows.
4. Audit-chain entries referencing this worker get a tombstone marker (NEW `worker_deleted_at` column on relevant audit rows) but the chain hash links remain intact for tamper-evidence.

Sibling fn `force_delete_worker(worker_id, tenant, confirm: bool)` that skips the grace gate for compliance-driven deletes (`argo compliance delete --tenant` integration — already exists per `claude-compliance-gdpr-data-delete` SHIPPED #1259). Both flow through the same SQL transaction.

This is NOT in scope: deleting non-archived (live) workers — operators must archive first; changing the archive table schema; deleting audit-chain entries (only tombstoning); deleting `org_member` rows (separate flow).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/worker_archive.rs` — narrowly scoped: ADD `pub async fn delete_worker(...)` + `pub async fn force_delete_worker(...)`. Both in one transaction. NO change to existing fns. File currently ~260 LOC — keep under 500.
- `crates/argo-store/src/worker_archive_tests.rs` — narrowly scoped: ADD 6 unit tests: delete-rejects-live-worker; delete-rejects-before-grace; delete-succeeds-after-grace; delete-cascades-memory-rows; delete-tombstones-audit-rows; force-delete-skips-grace.
- `crates/argo-cli/src/handlers/worker_archive.rs` — narrowly scoped: ADD `argo worker delete <worker_id> [--force]` CLI subcommand. NO change to existing `argo worker archive` / `argo worker rehydrate`.
- `crates/argo-cli/src/handlers/worker_archive_tests.rs` — narrowly scoped: ADD 2 CLI tests covering the new subcommand happy/error paths.

## Files you must NOT touch

- `crates/argo-store/src/worker.rs` (or wherever live `worker` table lives) — live-worker table is upstream; reuse only
- `crates/argo-audit/**` — audit-chain hashing is upstream; reuse only
- Any non-store/non-cli crate
- `crates/argo-store/migrations/**` — only ADD a new migration IF audit-row `worker_deleted_at` column is missing (verify first; if column already exists, no new migration)

## Acceptance criteria

- [ ] `delete_worker(worker_id, tenant)` exists, async, returns `ArgoResult<()>`
- [ ] Rejects with explicit error when worker_id is in live `worker` table (not archived)
- [ ] Rejects with explicit error including remaining-days when archive is younger than grace_window
- [ ] Default grace_window = 30 days; overrideable via `ARGO_WORKER_DELETE_GRACE_DAYS`
- [ ] Cascades per-tenant: memory rows tagged worker_id, skill quality scores keyed worker_id — never cross-tenant
- [ ] Audit-row references get `worker_deleted_at` tombstone but chain hash links preserved
- [ ] `force_delete_worker(worker_id, tenant, true)` skips grace gate (compliance path)
- [ ] `argo worker delete <id>` CLI works; `--force` flag plumbed
- [ ] 6 unit tests + 2 CLI tests
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-cli` clean
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-identity-delete-worker`
- PR title: `identity: IdentityStore::delete_worker with grace-window + cascade (mission-identity-delete-worker)`

## Notes for the implementer

- The grace window exists because operators sometimes archive in error and need a recovery window. 30 days is the brief's default — make it configurable so compliance teams can shorten it.
- Tombstoning the audit chain rather than deleting preserves tamper-evidence guarantees from `argo-audit`. The chain hash must not break — only add a `worker_deleted_at` field, never delete the row itself.
- The `force_delete_worker` path is what `argo compliance delete --tenant` calls. Reuse the existing GDPR cascade plumbing from #1259 if it touches the same tables.
- One SQL transaction wrapping both archive-row DELETE + cascade DELETEs + tombstone updates. If any step fails, roll back.
