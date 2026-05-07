# Worker presence tracking + `argo worker fleet`

**Owner**: claude. **Depends on**: —

## Why

To answer the operator question "which of my workers is actually
working right now?" we need a `last_active_at` column. With it, a fleet
view becomes trivial: list workers ordered by recency. Without it, the
question can't be answered without scanning every audit row in the
system. Backs mission claim 1 ("persistent identity over time" needs an
*observable* notion of time-since-active).

## Scope

- Schema: `worker.last_active_at INTEGER NULL` (unix epoch seconds).
- Update sites: any code path that creates a session for a worker, OR
  writes an audit row tagged with a worker_id, MUST update
  `last_active_at = strftime('%s','now')` for that worker.
- New CLI `argo worker fleet [--limit N] [--since <duration>] [--json]`
  listing workers ordered by `last_active_at DESC`.
- "Idle" rendering: workers with `last_active_at` more than 30 days old
  are flagged "idle" in markdown render.

## Approach (concrete)

1. **NEW** migration `crates/argo-store/migrations/32__worker_presence.sql`
   (~10 LOC): `ALTER TABLE worker ADD COLUMN last_active_at INTEGER`
   (nullable; older rows = NULL = "unknown").
2. **NEW** `crates/argo-store/src/worker_presence.rs` (~120 LOC):
   - `pub async fn touch_presence(store, worker_id) -> ArgoResult<()>`
     — single UPDATE; idempotent
   - `pub async fn fleet(store, tenant_id, limit, since) -> ArgoResult<Vec<FleetRow>>`
   - 5 unit tests
3. **Modify** session-create site — narrowly, ONE call site only:
   search for the function that inserts a `Session` row with a
   `worker_id` field, add `touch_presence` after the INSERT in the
   same transaction. Verify: `grep -rn "INSERT INTO session\|insert_session" crates/argo-store/src/`.
4. **Modify** audit-row writer — narrowly, ONE call site only: where
   the audit row carries a `worker_id`, also call `touch_presence`.
   Same caveat — find ONE site, do not propagate.
5. **NEW** `crates/argo-cli/src/handlers/worker_fleet.rs` (~200 LOC):
   - markdown table render: WorkerId | Persona | last_active | status
   - JSON render
   - `--since 7d` / `--since 1h` parsed via existing duration parser
     (search for `parse_duration`)
6. **Modify** `crates/argo-cli/src/cli.rs` — 1 new
   `Worker::Fleet { limit, since, json }` variant.

## Files you may touch

- `crates/argo-store/migrations/32__worker_presence.sql` (NEW)
- `crates/argo-store/src/worker_presence.rs` (NEW, ≤ 200 LOC)
- `crates/argo-store/src/worker_presence_tests.rs` (NEW, ≤ 200 LOC)
- `crates/argo-store/src/lib.rs` (1 pub mod line)
- One session-write site (narrow — ≤ 5 lines)
- One audit-write site (narrow — ≤ 5 lines)
- `crates/argo-cli/src/handlers/worker_fleet.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/worker_fleet_tests.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod + dispatch arm)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — 1 enum variant)
- `crates/argo-cli/tests/worker_fleet_smoke.rs` (NEW)

## Files you must NOT touch

- Multiple session-write or audit-write sites — find ONE canonical
  funnel for each. If there are several and you can't pick, surface
  to conductor; do not propagate.
- `crates/argo-identity/` Worker struct — `last_active_at` is a
  presence concern, not an identity concern; the Worker struct stays
  unchanged.
- Other migrations.

## Acceptance criteria

- [ ] After `argo session start <worker>`, `last_active_at` is within 5s of now
- [ ] After any audit-row write tagged with a worker_id, `last_active_at` is within 5s of now
- [ ] `argo worker fleet` lists workers ordered by `last_active_at DESC`, NULLs last
- [ ] `argo worker fleet --limit 5` returns at most 5
- [ ] `argo worker fleet --since 7d` filters to last-active within 7 days
- [ ] Workers idle >30d render with status `"idle"`
- [ ] Older worker rows (NULL last_active_at) render as "(never seen)"
- [ ] Migration is idempotent (re-running is a no-op)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-presence-tracking`
- PR title: `worker: last_active_at + `argo worker fleet` (claude-worker-presence-tracking)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies.
- Migration number 32: verify with
  `ls crates/argo-store/migrations/ | sort -V | tail` before writing.
  If 32 is already taken, use the next available.
- ONE session-write site, ONE audit-write site. If your `grep` finds
  many, you've found the wrong layer — go higher in the call stack to
  the funnel that all the writers go through.
- 30-day idle threshold is a constant, not a config — `IDLE_THRESHOLD_SECS`.
- Pre-existing audit flake (per tick 60 amendment) does not block.
