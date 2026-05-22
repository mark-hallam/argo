# tool-session-search-model-callable — Wrap existing FTS5 session search as a tool

**Owner**: claude. **Depends on**: nothing (FTS5 query path already
shipped via `argo-cli/src/handlers/session_search.rs`). **Tier A —
Agent tool surface.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Hermes parity (and immediate worker leverage): workers should be
able to search their own past sessions for "did I do this before /
what did the user say last time about X". The FTS5 index already
exists and the CLI handler already queries it. This brief promotes
that query to a model-callable in-core tool. Thin wrapper, ~100
LOC of net new logic.

## Approach (concrete)

1. Read `crates/argo-cli/src/handlers/session_search.rs` end-to-end.
   Identify the function that performs the FTS5 query against
   `argo-store` and returns hits. That same function is the call
   target.
2. Create `crates/argo-tool/src/tools/session_search.rs` exposing
   `session_search(query: string, limit?: u32, since_minutes?: u32)`.
   Defaults: `limit = 20`, `since_minutes = unbounded`. Returns
   `{ hits: [{ session_id, message_id, role, ts, snippet }] }`.
3. **Reuse, don't reimplement.** If the existing query function
   isn't pub-callable from outside `argo-cli`, lift the smallest
   possible function into `argo-store` (or wherever the FTS5 access
   layer lives) and have BOTH the CLI handler and the new tool call
   it. Keep the lift surgical — moving query SQL + result-row decode
   only, not CLI argument parsing or output formatting.
4. Register in `crates/argo-tool/src/tools/mod.rs`. Add the relevant
   crate (`argo-store` or new shared crate) to `argo-tool/Cargo.toml`
   if not already a dep.
5. Add to two vertical manifests (any two — this is broadly safe).
   Capability gating uses existing
   `Capability::ToolDispatch{tool_name_glob: "session_search"}`.
6. Tests:
   - 3+ unit: empty-result query, query with results, `since_minutes`
     boundary filter.
   - 1 integration: seed an in-memory `argo-store`, write three
     sessions with known content, query, assert exact-match hit
     ordering.
7. Document in the tool's rustdoc that the search is read-only and
   does NOT cross tenant boundaries (the existing FTS5 path already
   honors `tenant_id`; verify and document).

## Files you may touch

- `crates/argo-tool/src/tools/session_search.rs` (NEW, ≤ 200 LOC)
- `crates/argo-tool/src/tools/mod.rs` (mod + register in two arrays)
- `crates/argo-tool/src/lib.rs` (re-export `SessionSearch`)
- `crates/argo-tool/Cargo.toml` (add `argo-store` dep if missing)
- `crates/argo-store/src/` — only if a small lift is needed to make
  the FTS5 query accessible (≤ 60 LOC of net change; otherwise leave
  alone)
- `crates/argo-cli/src/handlers/session_search.rs` — only if you
  refactored it to call the lifted function (delta should be small)
- Up to two vertical manifests (single-line `allowed` edits each)
- `crates/argo-tool/tests/session_search_integration.rs` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- Any other CLI handler
- Any migration file
- `crates/argo-types/src/capability.rs`
- Other tool files

## Acceptance criteria

- [ ] Capability gating documented (existing `ToolDispatch`)
- [ ] At least one vertical's manifest allows `session_search`
- [ ] 3+ unit tests
- [ ] 1 integration test
- [ ] Rustdoc with example
- [ ] Tenant isolation verified + noted in rustdoc
- [ ] `cargo clippy -p argo-tool -p argo-store -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool -p argo-store -p argo-cli` green
- [ ] Workspace clippy final-sanity clean
- [ ] Public API documented; if a function was lifted, both old and
      new call sites pass the same tests as before

## Branch + PR

- Branch: `tool-session-search-model-callable`
- PR title: `tool-session-search-model-callable — model-callable wrapper for FTS5 session search`
