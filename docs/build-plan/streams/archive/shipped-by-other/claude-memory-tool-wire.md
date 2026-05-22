# Claude — wire `memory` tool to MemoryStore (vertical-aware)

**Owner**: claude. **Depends on**: F-memory-layering (#786),
C-mcp-wire (#788). Re-evaluation of stale PR #775; approach
validated, re-implementing on fresh main with vertical-scope fix.

## Scope

`Memory::dispatch()` returns `"memory store not wired"` regardless
of input — the agent runner never threaded a store handle through
tool-dispatch. LLMs in `argo chat` therefore can't write memory.

Wire `memory.list / read / write` through the live `MemoryStore`,
**honouring the bound vertical**: when the session is bound, agent
writes must go through `MemoryStore::append_for_vertical(_, _, _, _, vid)`
so vertical-scoped reads (`list_for_vertical`) see them. Unbound
sessions stay workspace-global (NULL `vertical_id`). Writes against
`Lock::UserProtected` rows reject with a clear error mentioning
`argo memory edit --force`.

## Files you may touch

- `crates/argo-tool/src/tools/memory.rs` — wire dispatch through
  `MemoryStore`; carry `Option<String>` vertical scope; route writes
  via `append_for_vertical` when set, else `append`/`supersede`
- `crates/argo-tool/src/tools/memory_tests.rs` — (NEW) sibling
  tests (keeps `memory.rs` < 500 LOC)
- `crates/argo-tool/src/tools/mod.rs` — add
  `builtin_tools_with_memory(cwd, store, ws, vertical_id)`
- `crates/argo-tool/src/lib.rs` — re-export new constructor
- `crates/argo-tool/Cargo.toml` — add `argo-memory` + `argo-store`
  path deps. **Do NOT add `uuid`** — `Uuid` arrives via `argo-types`
- `crates/argo-agent-runner/src/dispatch.rs` — extend
  `build_dispatch_ctx(cwd)` to accept
  `memory: Option<(Arc<MemoryStore>, WorkspaceId, Option<String>)>`.
  Keep MCP pool + `register_manifest_mcp_servers` intact
- `crates/argo-agent-runner/src/turn.rs` — in
  `run_turn_with_tools`, resolve workspace via
  `store.first_workspace().await?` and pass `vertical_scope`
  (already a parameter at L186)
- `crates/argo-agent-runner/Cargo.toml` — add `argo-memory` dep
- `Cargo.lock` — auto

## Files you must NOT touch

- `crates/argo-memory/**` — locked from F-memory-layering
- `crates/argo-store/**` — schema locked
- Anything not on the may-touch list

## Acceptance criteria

- [ ] `memory.write` through dispatch succeeds (not "not wired")
      when registered via `builtin_tools_with_memory`
- [ ] Vertical-bound write produces `vertical_id = Some(<id>)`;
      verified by reading back via `list_for_vertical`
- [ ] Unbound write produces `vertical_id = NULL`
- [ ] Write against `Lock::UserProtected` row errors clearly,
      mentions `--force`
- [ ] `Default::default()` Memory still surfaces the "not wired"
      error (back-compat for 15 existing `builtin_tools(cwd)` sites)
- [ ] No `uuid` dep added to `argo-tool/Cargo.toml`
- [ ] `cargo test -p argo-tool -p argo-agent-runner -p argo-cli` green
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file under 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-memory-tool-wire`
- PR title: `tools: wire memory tool through MemoryStore with vertical scoping (claude-memory-tool-wire)`

## Notes for the implementer

- PR #775 is the wiring-shape reference. Reuse the
  `Option<Arc<MemoryStore>>` + parallel constructor pattern; the
  bug is the missing vertical_id thread, not the architecture.
- `entry` accepts canonical (`MEMORY.md`) + bare (`memory`, `USER`)
  forms case-insensitively (match #775).
- Hoisting `MemoryStore::new` out of per-turn path is nice-to-have
  but expands scope; defer.
