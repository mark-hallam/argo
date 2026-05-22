# Mission load-bearing wire — agent-callable `memory.query_scored` action

**Owner**: claude. **Depends on**: `MemoryStore::query_scored` exists (verified at `argo-memory/src/layers.rs:182`); `argo-tool::tools::memory::Memory` exists with wired/unwired pattern (verified at `argo-tool/src/tools/memory.rs`).
**Authored against**: origin/main `9fe4c612` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-memory-query-scored-action`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 1 item #42 ("Add per-worker memory tool action `query_scored`"). Today the `memory` tool exposes `append` / `list_active` actions but NOT the `query_scored` retrieval primitive that already exists in `MemoryStore`. Workers can write to memory but can only read it back via `list_active` (filter-only, unranked). Mission Claim 1 ("real teammate") needs the agent to ask "what's the most relevant memory for THIS query?" — that's `query_scored`, with cosine-similarity ranking, already implemented at the store layer.

## Scope

Extend the `memory` tool (`argo-tool/src/tools/memory.rs`) with a `query_scored` action that takes:

- `query: String` — the search text (required)
- `limit: Option<usize>` — top-k cap (default 5, hard max 20)
- `namespace: Option<String>` — when None, queries the worker's bound namespace; when Some, queries the named namespace IFF the worker has read access (per `MemoryNs` policy)

The action returns a `Vec<MemoryHit>` envelope: `{ entry_id, snippet, score, namespace, written_at }` per hit. Reuses the existing `MemoryStore::query_scored` from `layers.rs:182` — no upstream change. The wired form receives the call; the default-stub returns the existing "memory store not wired" error.

This is NOT in scope: cross-tenant queries; building a UI for ranked memory; modifying `query_scored` ranking semantics; persisting query history.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-tool/src/tools/memory.rs` — extend `Memory::dispatch` to handle the `query_scored` action variant; add a typed `QueryScoredArgs` struct via `#[derive(Deserialize)]`; build hit envelope from `query_scored` results.
- `crates/argo-tool/src/tools/memory_tests.rs` — narrowly scoped: append 6 NEW unit tests beneath the existing test module: query-scored-default-namespace; query-scored-explicit-namespace; query-scored-respects-limit; query-scored-clamps-limit-to-20; query-scored-unwired-errors-with-historical-message; query-scored-on-namespace-without-access-errors.
- `crates/argo-tool/Cargo.toml` — narrowly scoped: confirm `argo-memory` is in `[dependencies]` (it is, transitively via `Memory` already importing types). NO other Cargo edits.
- `crates/argo-tool/tests/memory_query_scored_smoke.rs` (NEW) — integration smoke: seed a `MemoryStore` with 5 entries having distinct content; invoke `query_scored` via the tool surface; assert hits ranked by score and limited to 3.

## Files you must NOT touch

- `crates/argo-memory/**` — `query_scored` and `MemoryNs` policy are upstream contracts; reuse, don't modify
- Any other tool in `argo-tool/src/tools/`
- `crates/argo-store/migrations/**` — no schema change
- `crates/argo-cli/**` — no CLI exposure in this brief; the action is agent-callable only

## Acceptance criteria

- [ ] `Memory::dispatch` recognises a `query_scored` action with typed `QueryScoredArgs { query, limit, namespace }`
- [ ] When wired, the action delegates to `MemoryStore::query_scored(ns, query, limit.unwrap_or(5).min(20)).await`
- [ ] Hit envelope shape: `{ entry_id, snippet, score, namespace, written_at }` (snake_case JSON)
- [ ] When `limit > 20`, silently clamps to 20 (does NOT error — the cap is a guard, not a schema check)
- [ ] When `limit == 0`, returns an empty Vec (does NOT error)
- [ ] When unwired (`Memory::default()`), returns the EXISTING "memory store not wired" error verbatim — same string that today's `append` / `list_active` actions return
- [ ] When `namespace` is set to a name the worker has no read access to, surfaces a typed access error (reuse `argo-memory`'s existing access-deny path; do NOT introduce a new error variant)
- [ ] 6 new unit tests cover the matrix above
- [ ] 1 integration smoke covers seed-and-rank end-to-end
- [ ] `cargo build -p argo-tool` clean
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-memory-query-scored-action`
- PR title: `tool: agent-callable memory.query_scored action with namespace + limit (mission-memory-query-scored-action)`

## Notes for the implementer

- `MemoryStore::query_scored` signature: verify in first 5 minutes — `pub async fn query_scored(&self, ns: &MemoryNs, query: &str, k: usize) -> ArgoResult<Vec<ScoredHit>>`. If the actual signature differs (e.g. takes a tenant_id separately), adapt.
- The `namespace: Option<String>` parameter resolves through `MemoryNs::parse_or_worker_default(...)` style logic — check whether `MemoryNs` has a parser that accepts both bare `worker:<id>` and `shared:<team>` forms. If not, scope the v1 to `worker_default` only and BLOCK on `mission-memory-namespace-addressing` (#43, separate brief) for cross-namespace.
- Hit envelope: build by mapping `ScoredHit` → JSON serde struct. Don't expose the internal `ScoredHit` directly — it may carry implementation-detail fields (vector, raw row id) that shouldn't be agent-visible.
- The 20-limit cap: this is a soft guard against runaway prompts. Document in the action's rustdoc.
- If the existing `dispatch` method in `memory.rs` is approaching the 500-LOC budget after this addition, factor the new arm into a sibling `memory_query_scored.rs` (same crate, sibling module) with a narrow `pub(super) async fn dispatch_query_scored(...)` and ONE `pub(crate) mod memory_query_scored;` line in `tools/mod.rs`. If file size headroom is OK, keep inline.
