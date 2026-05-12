# Mission load-bearing wire — enterprise audit full-text + per-dim index

**Owner**: claude. **Depends on**: J14 (MERGED #1419) audit-feed endpoint that filters at the app layer over `payload_json`.
**Authored against**: origin/main `f1fa23f3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-enterprise-audit-full-text-search`.

> SOURCE: J14 #1419 PR-deferral. The audit-feed endpoint currently does a `created_at`-bounded SQL fetch (5000-row cap) and then filters in Rust over `payload_json` for all 7 filter dimensions (worker, channel, action_type, status, approver, cost_band, classification) and the `?q=` substring. That works at v1 single-tenant scale but won't survive the enterprise traffic profile (>10k audit rows/day per tenant). This brief promotes the filter dimensions to first-class columns + adds the composite indexes the J14 brief originally specified.

## Scope

Migration 64 adds the 7 filter columns to `audit_record` as nullable backfill-on-write (we don't backfill historical rows — the v1 audit chain is hashed and a backfill would either invalidate the chain or require a parallel-table approach we don't want yet). The audit-feed endpoint flips from app-layer to SQL-level filtering when the column is present; falls back to the existing `payload_json` parse for legacy rows where the column is NULL.

Specifically:

1. **Migration 64** — `ALTER TABLE audit_record ADD COLUMN worker_id TEXT`, `channel TEXT`, `action_type TEXT`, `outcome_status TEXT`, `approver_id TEXT`, `cost_band TEXT`, `data_classification TEXT`. Plus 3 composite indexes: `(created_at, worker_id)`, `(created_at, action_type)`, `(created_at, outcome_status)` covering the dominant filter combos. No `tenant_id` — that ships in the sibling brief `mission-enterprise-cross-tenant-audit`.
2. **Write path** in `argo-store::AuditStore::append` populates the new columns from the same `payload_json` fields the J14 reader currently parses. The hash chain stays on the existing 5 hashed columns (`id, session_id, kind, payload_json, parent_hash`) — these new columns are denormalized, NOT part of the hash. Add a sanity check in `verify_chain` that the denorms match the JSON.
3. **Read path** in `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` adds a branch: if the migration is applied (detect via `PRAGMA table_info`), build a SQL `WHERE` clause directly. Otherwise keep the existing app-layer pass for compatibility with un-migrated DBs (downgrade safety).
4. **`?q=` substring** stays app-layer for v1 (SQLite `LIKE` is fine on 10k rows; full-FTS5 promotion is closed by a follow-up `mission-enterprise-audit-fts5-index`).
5. **Bench**: add `crates/argo-cli/benches/audit_feed.rs` (criterion) seeding 50k rows and asserting the SQL path is < 50ms p99 for a single-dim filter.

This is NOT in scope: tenant_id column (sibling brief); FTS5 virtual table (separate brief); backfill of historical rows (chain integrity); deleting the app-layer filter (kept as fallback); auditing the audit-row writes themselves.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/64__audit_record_filter_columns.sql` (NEW, ≤ 80 LOC) — 7 columns + 3 indexes (slot 63 taken by J16 elevated_view_grant #1426 2026-05-12)
- `crates/argo-store/src/audit.rs` (≤ 480 LOC after) — write path populates new columns from `payload_json`; `verify_chain` denorm sanity check
- `crates/argo-store/src/audit_tests.rs` (NEW or extend, ≤ 350 LOC) — 5 tests: column populated on write, downgrade-safe (NULL columns), verify_chain catches denorm drift, index used (EXPLAIN QUERY PLAN), no chain hash impact
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (≤ 480 LOC after) — SQL branch + fallback
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs` (≤ 480 LOC after) — extend with 3 tests: SQL path returns same shape as app-layer path; index used; legacy NULL rows fall through to app-layer parse
- `crates/argo-cli/benches/audit_feed.rs` (NEW, ≤ 220 LOC) — criterion harness
- `crates/argo-cli/Cargo.toml` — add `[[bench]]` entry (additive)

## Files you must NOT touch

- `crates/argo-store/migrations/0..63__*.sql` — locked (63 = J16 elevated_view_grant)
- `crates/argo-cli/src/handlers/gateway/jwt*.rs` — orthogonal to this brief
- Audit chain hash logic — promoting columns to hashed would invalidate every historical row's parent_hash

## Acceptance criteria

- [ ] Migration 64 applies cleanly forward + on a DB that already has rows (NULLs for legacy)
- [ ] New columns populated on subsequent writes; NULL for historical rows
- [ ] `verify_chain` flags drift between JSON `payload_json.worker_id` and denorm `worker_id` column with a typed error
- [ ] Hash chain unchanged — re-hashing a row with the new columns produces the same `self_hash`
- [ ] `routes_audit_feed` SQL path used when migration applied; app-layer fallback used when column is NULL
- [ ] Both paths produce byte-identical JSON for a given filter set (property test)
- [ ] criterion bench: single-dim SQL filter < 50ms p99 over 50k rows
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-enterprise-audit-full-text-search`
- PR title: `audit: enterprise filter columns + indexes (mission-enterprise-audit-full-text-search)`

## Notes for the implementer

- Migration slot 63 was taken by J16 (PR #1426 elevated_view_grant, 2026-05-12) — this brief uses slot 64. Verify the next free slot before pushing.
- The chain-hash invariant is the most load-bearing constraint here. The new columns must be additive denormalizations — NEVER part of the input to the parent_hash → self_hash chain. The denorm sanity check in `verify_chain` is the safety belt that catches a future maintainer accidentally adding the columns to the hashed set.
- For the `?range=` symbolic presets (1h, today, yesterday, 7d, 30d, qtd, lq) — these stay on `created_at`, which already has the migration 51 index implicitly. No new index needed for range filtering.
- The criterion bench is the brief's ship gate — without it, the "this scales to enterprise traffic" claim is just an assertion. Use a fixed seed + tmpfile DB so the bench is hermetic.
- Document in `routes_audit_feed.rs` module docs that the SQL path is preferred but the app-layer path remains the source of correctness — if the two ever diverge, the app-layer is right.
- Companion brief `mission-enterprise-cross-tenant-audit` adds `tenant_id` (migration 65) and the 403 cross-tenant gate. Don't try to do both in one PR. (That brief is currently BLOCKED awaiting conductor rewrite — premise mismatch against actual crate layout.)
