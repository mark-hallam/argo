# Stream F (follow-up) — `MemoryNs::shared_team` namespace

**Owner**: codex. **Depends on**: F-memory-layering (#786) merged.

## Scope

Add a tenant-scoped shared memory namespace that every worker on
the team can read + write. Writes are stamped with the author's
`worker_id` so the read side can show who wrote what.

Mentioned as out-of-scope in the H-3 PR #799 ("MemoryNs::shared_team
— touches argo-memory which is owned by Stream F"). This brief
ships that piece.

## Files you may touch
- `crates/argo-memory/src/lib.rs` — narrowly scoped: ONE
  re-export line + at most ONE new `pub mod` declaration.
- `crates/argo-memory/src/shared_team.rs` (NEW)
- `crates/argo-memory/src/store.rs` — narrowly scoped: extend
  with `read_shared_team` / `write_shared_team` methods. Do NOT
  modify any existing method.
- `crates/argo-store/migrations/15__shared_team_memory.sql`
  (NEW migration if a new table is needed; alternatively reuse
  the existing memory table with a discriminator column added
  via a `15__alter_memory.sql` migration — pick whichever is
  cleaner).
- `Cargo.lock`

## Files you must NOT touch
- `crates/argo-identity/**` — `MemoryNs` lives there but the
  shared-team constructor goes in `argo-memory` (the storage
  layer owns its namespace shapes). If a new method on
  `MemoryNs` is genuinely needed, ask in BUILD_STATUS first.
- Any other crate.

## Acceptance criteria
- [ ] `MemoryNs::shared_team(tenant_id)` (or equivalent free
      function in `argo-memory::shared_team`) constructs a
      namespace shared by every worker in the tenant.
- [ ] `write_shared_team(ns, author_worker_id, content)`
      records the write with the author stamp.
- [ ] `read_shared_team(ns)` returns `Vec<SharedEntry>` where
      each entry includes `author_worker_id`, `content`,
      `written_at`.
- [ ] Two workers in the same tenant see each other's writes.
- [ ] Two workers in DIFFERENT tenants do NOT see each other's
      writes (cross-tenant isolation invariant).
- [ ] Migration applies cleanly on a fresh DB AND on an
      existing DB that has the F-memory-layering migrations.
- [ ] 4+ unit tests:
      - same-tenant cross-worker read sees writes
      - cross-tenant isolation
      - author stamp survives the round-trip
      - migration is idempotent (applying twice is a no-op or
        rejects cleanly)
- [ ] `cargo clippy -p argo-memory --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-F-memory-shared-team`
- PR title: `memory: shared-team namespace with author stamping (F-memory-shared-team)`

## Notes
- Look at `crates/argo-memory/src/layers.rs` for how F-memory-
  layering structured its types — mirror the shape rather than
  inventing a new pattern.
- The author stamp must be queryable, not just stored. The
  read API returns `(content, author_worker_id, when)` tuples
  so the orchestrator's session-history view (H-3) can label
  who said what.
- Cross-tenant isolation is a security invariant — the unit
  test for it is the most important one in the suite. Make it
  unambiguous.
