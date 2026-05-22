# Stream H (follow-up) — Orchestrator sqlite persistence

**Owner**: codex. **Depends on**: H-3 (#799) merged.

## Scope

Sqlite persistence for `Session` + `Handoff` types from
`argo-orchestrator`. Currently the types exist but live only in
RAM — the gateway / dispatch path can't yet hand off across
worker ticks because there's no store.

This brief adds:
- A migration creating `session` + `handoff` tables.
- A `persist::OrchestratorStore` repository on top of the
  existing `argo_store::SessionStore` connection pool.
- CRUD methods needed by H-3's downstream consumers:
  `insert_session`, `find_session`, `list_active_sessions`,
  `update_session_status`, `add_participant`,
  `insert_handoff`, `find_handoff`, `list_handoffs_for_session`,
  `list_pending_handoffs_for_worker`, `update_handoff_status`.

## Files you may touch
- `crates/argo-orchestrator/src/persist.rs` (NEW)
- `crates/argo-orchestrator/src/lib.rs` — narrowly scoped: ONE
  new `pub mod persist;` line + ONE `pub use` block for the
  store type. Do NOT modify the existing module list or
  re-exports.
- `crates/argo-orchestrator/Cargo.toml` — add `argo-store`
  dependency + `sqlx` (workspace).
- `crates/argo-store/migrations/15__orchestrator.sql` (NEW —
  unless F-memory-shared-team takes 15, in which case use 16).
  Coordinate via BUILD_STATUS if both land at once: first to
  push picks 15, the other rebases to 16.
- `Cargo.lock`

## Files you must NOT touch
- `crates/argo-orchestrator/src/session.rs`
- `crates/argo-orchestrator/src/handoff.rs`
- `crates/argo-orchestrator/src/escalation.rs`
- `crates/argo-cli/**` — CLI surface lands in
  `H-orchestrator-cli` (separate brief).
- `crates/argo-gateway/**` and any gateway routes file — HTTP
  surface lands in `H-orchestrator-routes` (separate brief).

## Acceptance criteria
- [ ] Migration creates `session` table with columns
      (`id`, `tenant_id`, `title`, `status`, `participants`
      (JSON), `created_at`, `deadline`).
- [ ] Migration creates `handoff` table with columns
      (`id`, `session_id`, `from_worker_id`, `to` (JSON for
      the `EscalationStep`), `expected_outcome`, `payload`
      (JSON), `status`, `created_at`, `deadline`).
- [ ] `OrchestratorStore::new(store: Arc<SessionStore>)`.
- [ ] All CRUD methods listed in §Scope land + return
      `ArgoResult` where appropriate.
- [ ] `update_session_status` / `update_handoff_status`
      enforce the lifecycle invariants (no Resolved →
      Active backslide; no Settled → Pending).
- [ ] Round-trip test: insert a session + 2 handoffs, read
      them back, status fields match.
- [ ] Cross-tenant isolation test: a session inserted with
      tenant A is NOT visible to a `list_active_sessions(B)`
      call.
- [ ] 6+ unit tests with in-memory sqlite (`open(":memory:")`).
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-orchestrator-persist`
- PR title: `orchestrator: sqlite persistence for sessions + handoffs (H-orchestrator-persist)`

## Notes
- For the `participants` field, store as a JSON array of
  worker-id strings; deserialise into `Vec<WorkerId>` on read.
- For `to: EscalationStep`, serialise as the existing
  `#[serde(tag = "kind")]` JSON the type already produces.
- Use `sqlx::query!` macro at compile time for the queries —
  the workspace already uses it elsewhere.
- See `crates/argo-skill/src/store.rs` for the migration +
  CRUD shape pattern; mirror it.
