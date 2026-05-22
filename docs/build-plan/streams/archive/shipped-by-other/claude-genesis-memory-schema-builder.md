# Genesis memory schema builder — namespace structure from role description

**Owner**: claude. **Depends on**: none (extends existing Genesis crate).

## Scope

Every vertical needs a memory schema: `memory_namespaces` table rows
plus a per-role layered structure (worker / team / org). Today the
operator manually authors `memory-schema.toml` per vertical. This brief
generates a default schema from the role description that the operator
can refine.

Mirrors mission claim 3 (memory schema generation).

## Approach (concrete)

1. **NEW** `crates/argo-genesis/src/memory_schema.rs` (~280 LOC):
   - `struct GeneratedMemorySchema { namespaces: Vec<MemoryNamespace>, layers: Vec<MemoryLayer> }`
   - `struct MemoryNamespace { id: String, scope: NamespaceScope, ttl_days: Option<u32>, summary: String }`
   - `enum NamespaceScope { Worker, Team, Org }`
   - `struct MemoryLayer { name: String, namespaces: Vec<String>, retrieval_policy: String }`
   - `pub fn generate(role_description: &str, archetype: Option<&Archetype>) -> ArgoResult<GeneratedMemorySchema>`
     — emits 1 worker namespace by default; adds team namespace if role
     mentions "team" / "shared" / "handoff"; adds org namespace if role
     mentions "company" / "policy" / "compliance"
   - `pub fn to_toml(schema: &GeneratedMemorySchema) -> String`
2. **Extend** `lib.rs` to `pub mod memory_schema;`
3. Wire into `pipeline.rs` so `manifest_writer` consumes the schema
   when emitting the vertical TOML.

## Files you may touch

- `crates/argo-genesis/src/memory_schema.rs` (NEW, ≤ 320 LOC)
- `crates/argo-genesis/src/memory_schema_tests.rs` (NEW, ≤ 220 LOC) — 5 tests:
  - solo role → 1 namespace, scope=Worker
  - role mentioning "team" → 2 namespaces (Worker + Team)
  - role mentioning "policy" → 3 namespaces (Worker + Team + Org)
  - layers always include at least 1 retrieval policy
  - to_toml() round-trips through `toml::from_str`
- `crates/argo-genesis/src/lib.rs` (narrowly: 1 pub mod line)
- `crates/argo-genesis/src/pipeline.rs` (narrowly: 1 call site)

## Files you must NOT touch

- `crates/argo-vertical/src/manifest.rs` — schema fields are existing
- The `memory_namespaces` DB table — schema-builder writes TOML only

## Acceptance criteria

- [ ] `generate()` always returns ≥ 1 namespace + ≥ 1 layer
- [ ] Worker scope is always present
- [ ] Detection of "team" / "policy" keywords lifts scope correctly
- [ ] `to_toml(...)` parses cleanly via `toml::from_str::<Value>`
- [ ] 5 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-genesis` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-memory-schema-builder`
- PR title: `genesis: build memory schema (namespaces + layers) from role description (claude-genesis-memory-schema-builder)`

## Notes for the implementer

- v0 keyword-based scope detection is fine — record a TODO for a
  classifier upgrade in a later PR.
- TTL defaults: Worker=30d, Team=90d, Org=None. Operators can override.
- Look at one of the demo verticals (e.g. `verticals/sales-sdr/`) to
  confirm the TOML shape memory_schema must produce.
