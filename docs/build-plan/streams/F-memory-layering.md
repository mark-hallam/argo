# Stream F — Memory layering + embeddings

**Owner**: codex. **Depends on**: E-identity merged (for
`MemoryNs`).

## Scope

Extend `argo-memory` from a single append-only file model to
three layers stored in the same `memory_entry` table with a
`layer` column: `recent` (last 30 days, full text, always in
context), `indexed` (FTS5 + sqlite-vec embeddings, retrieved
on demand), `summary` (monthly digests by Claude Haiku, always
included).

## Files you may touch
- `crates/argo-memory/**`
- `crates/argo-store/migrations/<next>__memory_layers.sql`
  (NEW)
- `crates/argo-cli/src/handlers/cron.rs` or a new
  `memory_summary` cron handler (NEW: nightly summarisation
  job)
- `Cargo.toml` (add `sqlite-vec` extension)

## Files you must NOT touch
- `argo-identity` types (locked by Stream E)
- Any vertical's manifest
- Other stream files

## Acceptance criteria
- [ ] `memory_entry` schema gains `layer TEXT NOT NULL DEFAULT 'recent'`
- [ ] `MemoryStore::append()` defaults rows to `recent`; a
      background job promotes `recent` rows older than 30 days
      to `indexed`
- [ ] `MemoryStore::query(ns, query, k)` retrieves top-k from
      `indexed` via FTS5 + vector similarity (sqlite-vec
      hybrid scoring)
- [ ] `compose_context(worker_id, action_id) -> ContextPack`
      composes: all `recent` rows + all `summary` rows + top-k
      `indexed` rows for the action's keywords. Fits in <8k
      tokens by design.
- [ ] Embeddings via Voyage-3 model through the existing
      provider abstraction (add a new family if needed)
- [ ] Nightly summary job: runs at 2am workspace-local; reads
      last month's `recent` rows per worker, calls Haiku, writes
      a single `summary` row per worker per month
- [ ] Property test: cross-tenant query returns ZERO rows from
      another tenant's `MemoryNs`
- [ ] `cargo clippy -p argo-memory -- -D warnings` clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-F-memory-layering`
- PR title: `memory: 3-layer storage (recent/indexed/summary) + sqlite-vec + Voyage-3 embeddings (F-memory-layering)`

## Notes
- sqlite-vec is a cargo-installable extension; load at
  connection open.
- Voyage-3 is currently the best non-OpenAI embedding model
  for general text; if your impl uses a different provider
  abstraction, document the choice.
- Summary job is naive in v1: full-month bucket → one Haiku
  call → one row. Optimize later.
