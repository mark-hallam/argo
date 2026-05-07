# Genesis action suggestor — top-K action vocabulary from role description

**Owner**: claude. **Depends on**: none (extends existing Genesis crate).

## Scope

Genesis must suggest a coherent **action vocabulary** for the worker
based on the role description, by inspecting the connector library
(MCP connectors under `crates/argo-mcp-connector-*`) and the skill
library (verticals/<name>/skills/). Today the operator manually picks
actions in `manifest_writer.rs`. This brief automates the picking.

Mirrors mission claim 3 (action vocabulary generation).

## Approach (concrete)

1. **NEW** `crates/argo-genesis/src/action_suggest.rs` (~300 LOC):
   - `struct SuggestedAction { id: String, source: ActionSource, score: f32, reason: String }`
   - `enum ActionSource { Connector(String), Skill(String) }`
   - `pub fn suggest(role_description: &str, top_k: usize) -> ArgoResult<Vec<SuggestedAction>>`
     — embeds role description, scores against pre-computed embeddings
     of connector/skill summaries, returns top-K
   - `pub fn enumerate_connectors() -> ArgoResult<Vec<ConnectorSummary>>`
     — reads `crates/argo-mcp-connector-*` `Cargo.toml` package
     descriptions + each crate's exported action list
   - `pub fn enumerate_skills(verticals_root: &Path) -> ArgoResult<Vec<SkillSummary>>`
     — walks `verticals/*/skills/*.toml`
2. **Extend** `lib.rs` to `pub mod action_suggest;`
3. Wire into `pipeline.rs` so the manifest writer calls suggest()
   before emitting `[[actions]]` blocks.

## Files you may touch

- `crates/argo-genesis/src/action_suggest.rs` (NEW, ≤ 320 LOC)
- `crates/argo-genesis/src/action_suggest_tests.rs` (NEW, ≤ 250 LOC) — 6 tests:
  - "sales SDR who emails prospects" → gmail/hubspot in top 5
  - "AR clerk reconciling invoices" → quickbooks/stripe in top 5
  - top_k = 0 → empty Vec, not error
  - unknown role → returns generic actions (web-search, scratchpad)
  - enumerate_connectors finds ≥ 5 connector crates
  - enumerate_skills finds ≥ 5 skills across the demo verticals
- `crates/argo-genesis/src/lib.rs` (narrowly: 1 pub mod line)
- `crates/argo-genesis/src/pipeline.rs` (narrowly: call suggest() in flow)

## Files you must NOT touch

- Any `argo-mcp-connector-*` crate — read-only inspection
- `verticals/*/` — read-only inspection
- `manifest_writer.rs` schema — only the call site changes

## Acceptance criteria

- [ ] suggest() returns ≥ 3 actions for any of the 5 demo verticals' role descriptions
- [ ] Each SuggestedAction has a non-empty `reason` field
- [ ] enumerate_connectors() picks up `argo-mcp-connector-gmail`, `-gcal`, `-hubspot`, `-slack`, `-notion`
- [ ] enumerate_skills() walks vertical TOML without panicking on a missing skills/ dir
- [ ] 6 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-genesis` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-action-suggestor`
- PR title: `genesis: top-K action suggestor from connector + skill libraries (claude-genesis-action-suggestor)`

## Notes for the implementer

- For embedding/similarity, prefer a deterministic keyword overlap +
  TF-IDF score in v0 (no model dep). A real embedding can land in a
  later PR — keep the API stable.
- Pre-compute connector summaries at startup (`OnceLock<Vec<ConnectorSummary>>`)
  — enumeration is read-only and cheap to cache.
- Score range [0.0, 1.0]; don't over-engineer — top-K = 5 is the
  default operator-facing case.
