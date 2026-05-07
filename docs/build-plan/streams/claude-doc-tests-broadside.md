# claude-doc-tests-broadside — workspace-wide doctest coverage on public boundary

**Owner**: claude. **Depends on**: claude-types-doc-improvements (lands first; this brief widens the same idea to the rest of the workspace). **Phase B doc-quality.**

## Why

`claude-types-doc-improvements` covers `argo-types` (the foundation crate). Other crates have the same gap: thin doc-comments on public methods, zero compile-checked examples. Skill consumers, integration writers, and contributors all learn the API by trial-and-error.

Doctests double as compile-checked examples + as a forcing function for usable APIs (if your example is awkward to write, the API is awkward).

Goal: every public type or method on the workspace boundary has at least one doctest.

## Approach (concrete)

**Workspace-wide narrow doc additions.** Zero behaviour change. Per-crate, add ≥ 1 doctest per public boundary symbol.

### Crate-by-crate plan

1. `argo-store` — public types: `Store`, `SessionRow`, `MessageRow`, `WorkerRow`, `EventRow`, `BudgetRow`. Public methods: `Store::open`, `Store::session_create`, `Store::message_append`, `Store::worker_inspect`, `Store::events_recent`. Doctests use a temp SQLite DB.
2. `argo-channels` — `ChannelId::new`, `Push::send`, `Push::recv`. Doctests use the in-memory channel impl.
3. `argo-marketplace` — `Bundle::parse`, `Bundle::install`, `Registry::lookup`. Doctests use a fixture bundle.
4. `argo-vertical` — `VerticalManifest::parse`, `VerticalManifest::validate_strict` (if landed), `Vertical::dispatch`. Doctests use minimal manifest strings.
5. `argo-memory` — `MemoryStore::put`, `MemoryStore::get`, `MemoryStore::list`. Doctests use the in-memory backend.
6. `argo-identity` — `Tenant::new`, `Worker::new`. Construction round-trips.
7. `argo-error` — `ArgoError` variants (covered by claude-error-types-consolidation if that lands first; otherwise covered here).
8. `argo-config` — `ArgoConfig::load_from`, `ArgoConfig::merge` (depends on claude-config-loader-overhaul if landed).
9. `argo-cli` — non-handler public API only (the handlers are internal).
10. `argo-agent-runner` — `Turn::execute` happy-path doctest with a stub provider.
11. `argo-tui` — public command/event types only (the rendering layer is internal).
12. `argo-skill` — `Skill::load`, `Skill::execute`.

For each: ≤ 10-line doctest demonstrating the canonical happy path. Use `# use` hidden setup lines liberally.

## Files you may touch

- Every `crates/<crate>/src/**/*.rs` file containing public types or methods on the workspace boundary — narrow `///` doc additions
- NO new files unless an existing file would exceed 500 LOC after additions (in which case extract to a sibling)

## Files you must NOT touch

- Any test file (the doctests live inline; no `tests/` changes)
- Any handler internal API
- Cargo.toml files (no new deps)

## Acceptance criteria

- [ ] Every public boundary type/method has at least 1 doctest
- [ ] `cargo test --workspace --doc` runs every doctest green
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] No public API change (rustdoc diff is purely additive)
- [ ] Every modified file remains ≤ 500 LOC (split if needed)
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-doc-tests-broadside`
- PR title: `docs: workspace-wide doctest coverage on public boundary (claude-doc-tests-broadside)`

## Notes for the implementer

- Stack this PR on top of `claude-types-doc-improvements` once that's merged. The argo-types examples become the reference style.
- Order the work crate-by-crate. Open one PR per crate **only if review feedback indicates the broadside is too large** — otherwise one PR is fine.
- For doctests that need a temp file (e.g. `Store::open`), use `tempfile::TempDir` (already a dev-dep). Hide the boilerplate with `#`.
- For doctests that need a stub provider/MCP server, use `# fn make_stub() -> StubProvider { todo!() }`-style hidden setup. Don't ship working stubs as part of the doctest if it'd bloat the example.
- If a public method is genuinely awkward to demonstrate in 10 lines, that's a signal to file a follow-up brief simplifying the API — do NOT contort the doctest.
- Prioritise crates by import-frequency. `argo-store`, `argo-channels`, `argo-vertical` are the heaviest consumers.
