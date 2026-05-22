# V1.4 — Handoff `resolved_at` schema migration

**Owner**: codex. **Depends on**: H-orchestrator-persist (#806) merged.
**Unblocks**: codex-handoff-stats-cli.

> **NEW PREREQUISITE BRIEF (tick 269 by conductor)**: the original
> `codex-handoff-stats-cli` brief was BLOCKED on a schema gap —
> the `handoff` table has only `created_at` + `deadline`, no
> `resolved_at`. Acceptance #1 of stats-cli requires "median
> latency from request to resolution", which needs a real
> resolution timestamp. This narrow prerequisite brief adds the
> column + writes from the persist module when a handoff settles.

## Scope

Add a `resolved_at INTEGER` column to the `handoff` table and
populate it from `argo-orchestrator/src/persist.rs` whenever a
handoff transitions to a terminal status (resolved / escalated /
abandoned). Once landed, `codex-handoff-stats-cli` unblocks
naturally.

Non-scope: any aggregate queries, CLI surface, or stats display.
Those land in the unblocked stats-cli brief.

## Approach

1. **NEW migration** in
   `crates/argo-store/migrations/` — add a new raw SQL file with
   the next free numeric prefix (current highest is 29; pick 30
   or higher per current convention). The file:
   ```
   ALTER TABLE handoff ADD COLUMN resolved_at INTEGER;
   CREATE INDEX IF NOT EXISTS idx_handoff_resolved_at
       ON handoff(resolved_at) WHERE resolved_at IS NOT NULL;
   ```
   > NOTE: this brief co-exists with the migration-reconciliation
   > sub-briefs. If `mission-migration-reconciliation-v28-37`
   > has already merged (no raw SQL on disk), instead add a
   > `vN.rs` versioned module at
   > `crates/argo-store/src/migrations/vN.rs` mirroring the
   > existing v11.rs shape. Implementer chooses the path based on
   > what's already on main when this brief is picked up.
2. **Modify** `crates/argo-orchestrator/src/persist.rs`:
   - Extend the `INSERT` query for handoff inserts (no change to
     the inserted row — `resolved_at` defaults to NULL)
   - Add an `update_handoff_resolved_at(handoff_id, now_micros)`
     function that fires from the `handoff_settled` audit-kind
     handler (or wherever the orchestrator records terminal
     status today)
3. **Wire the call** wherever `handoff_settled` audit kind is
   emitted today — narrowly. If the call site is unclear, the
   implementer should `grep handoff_settled` and pick the
   shallowest natural call site.
4. **Tests**: 3 unit tests
   - `update_handoff_resolved_at` writes the timestamp
   - re-update is idempotent (last-write-wins; no error)
   - `resolved_at` remains NULL for handoffs that have not
     settled

## Files you may touch

- `crates/argo-store/migrations/30__handoff_resolved_at.sql`
  (NEW — preferred path if raw SQL still in use; pick next
  available numeric prefix)
- OR `crates/argo-store/src/migrations/vN.rs` (NEW — if migration
  reconciliation has merged; mirror v11.rs shape)
- `crates/argo-store/src/migrations/mod.rs` — narrowly: register
  the new module if going the vN.rs path
- `crates/argo-orchestrator/src/persist.rs` — narrowly: add
  `update_handoff_resolved_at` + extend the existing audit-kind
  handler call site
- `crates/argo-orchestrator/tests/handoff_resolved_at_smoke.rs`
  (NEW — ≤ 200 LOC)
- BLANKET CLI ALLOWLIST does NOT apply (no CLI surface here)

## Files you must NOT touch

- Any other handoff-related SQL or CLI handler — this brief is
  schema + persist only
- The `handoff_routing_rule` table (unrelated; #883)

## Acceptance criteria

- [ ] `handoff` table has `resolved_at INTEGER` column (NULL by
      default)
- [ ] Index `idx_handoff_resolved_at` exists on `(resolved_at)
      WHERE resolved_at IS NOT NULL`
- [ ] `update_handoff_resolved_at(id, now_micros)` writes the
      timestamp; idempotent on re-call
- [ ] The `handoff_settled` audit-kind handler in
      `argo-orchestrator/src/persist.rs` calls
      `update_handoff_resolved_at` before returning
- [ ] 3 unit tests cover write / idempotency / NULL-default
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-orchestrator` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `codex-handoff-resolved-at-schema`
- PR title: `handoff: add resolved_at schema + persist write (codex-handoff-resolved-at-schema)`

## Notes for the implementer

- Look at the existing `add_column_if_missing` helper in
  `argo-orchestrator/src/persist.rs` line ~37 — it adds
  `deadline INTEGER` defensively; the same pattern works here as
  a defence-in-depth alongside the migration.
- Once this brief lands, ledger note for the dependent stats-cli
  brief should flip BLOCKED → TODO.
