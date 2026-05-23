# mission-pack-format-adapter-to-vertical-loader

**Priority**: P1 (claim-3 + claim-4 — bridge the `packs/` schema into the runtime loader so the marketplace cache becomes operationally visible)
**Owner**: claude
**Type**: Production adapter + integration test
**Size**: ~250-450 LOC across one new module + 1 caller wire-up + integration test
**Authored against**: origin/main `01bbe9b58` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pack-format-adapter-to-vertical-loader`

## Why

The evaluator audit `eval-vertical-pack-registry-coverage` (2026-05-23) identified **three parallel vertical-discovery surfaces** with no operational bridge:

1. `~/.argo/packs/` cache (`PackLoader::list`) — empty on fresh install, no bootstrap, no `argo packs search`. Only consumer is the `argo packs` CLI.
2. Repo `verticals/` directory (36 subdirs) — 32 loadable via `argo_vertical::load_dir` (`vertical.toml` schema).
3. Repo `packs/` directory (10 packs in `worker.toml` schema) — **0 consumed by any runtime path**; only `argo_skill::pack_lint` reads them.

The schemas overlap by id but differ structurally: a `packs/<id>/worker.toml` cannot be passed to `argo_vertical::load_dir` because the field layout, prompt-path conventions, and memory-schema declarations don't match. This blocks claim-3 ("The platform builds workers") from honouring the marketplace as a worker source — operators who install a `packs/` entry through `argo packs install` get a cache directory that no runtime path reads.

The gap manifest names this brief as the **F-2 fix** in its `argo-vertical-packs` row: `mission-pack-format-adapter-to-vertical-loader` — bridge `worker.toml` packs into `argo_vertical::load_dir` so the marketplace cache becomes operationally visible.

## What

### Step 1 — adapter module

New file `crates/argo-vertical-packs/src/adapter.rs` (~120-180 LOC):

```rust
//! Translate a `packs/` directory (worker.toml schema) into the
//! intermediate representation `argo_vertical::load_dir` returns
//! (`Vertical` struct), so a marketplace pack can be loaded by the
//! same runtime path that consumes a repo `verticals/<id>/` entry.

pub fn pack_dir_to_vertical(pack_root: &Path) -> Result<Vertical, AdaptError> {
    // 1. read worker.toml, parse via WorkerManifest::from_toml
    // 2. map fields: WorkerManifest.identity → Vertical.system_prompt
    //                WorkerManifest.tools     → Vertical.action_vocabulary
    //                WorkerManifest.memory    → Vertical.memory_schema
    //                WorkerManifest.eval      → Vertical.eval_seeds
    // 3. if pack_root has prompts/, splice into Vertical.prompts_dir.
    // 4. if mapping incomplete (e.g. pack has no eval seeds),
    //    return AdaptError::IncompleteMapping naming the missing
    //    fields. Callers decide whether to fail-closed.
    todo!()
}
```

The adapter is **read-only**: it must not write to the pack directory and must not mutate the `Vertical` after construction. Tests will round-trip.

### Step 2 — wire into PackLoader

`crates/argo-vertical-packs/src/loader.rs` already exposes `PackLoader::list`. Add a sibling:

```rust
impl PackLoader {
    pub fn load_as_vertical(&self, pack_id: &str) -> Result<Vertical, LoadError> {
        let pack_root = self.cache_dir.join(pack_id);
        if !pack_root.exists() { return Err(LoadError::NotInstalled(pack_id.to_string())); }
        adapter::pack_dir_to_vertical(&pack_root).map_err(LoadError::Adapt)
    }
}
```

### Step 3 — integration test

New file `crates/argo-vertical-packs/tests/pack_adapter_round_trip_test.rs` (~80-120 LOC, ~5 tests):

1. **Happy path** — install a fixture pack (use `packs/project-coordinator` as the seed; copy into a tempdir), call `PackLoader::load_as_vertical("project-coordinator")`, assert the returned `Vertical`:
   - has non-empty `system_prompt`,
   - declares ≥1 `action_vocabulary` entry,
   - the `memory_schema` parses through `argo_memory::Schema::validate`,
   - can be passed to `argo_vertical::load_dir`'s consumer interface (use a public helper or construct a `WorkflowDefinition` directly).
2. **Incomplete-mapping** — install a fixture pack with no `eval/` directory; assert `AdaptError::IncompleteMapping { missing: vec!["eval_seeds"] }`.
3. **Schema-mismatch** — install a pack whose `worker.toml` has an unknown field; assert the parse fails through `WorkerManifest::from_toml` (not silently dropped).
4. **Not-installed** — call `load_as_vertical("does-not-exist")`; assert `LoadError::NotInstalled`.
5. **Read-only contract** — install a pack, call `load_as_vertical` 3 times, assert pack-directory `mtime` is unchanged after each call.

### Step 4 — F-5 closure note

The audit also flags `packs/project-coordinator` (Peta) as having no `verticals/project-coordinator/` counterpart. This brief's adapter closes F-5 too: once the adapter works, Peta is loadable via the cache path instead of needing a parallel `verticals/` entry.

## Acceptance

- [ ] `crates/argo-vertical-packs/src/adapter.rs` exists with the public `pack_dir_to_vertical` fn + `AdaptError` enum.
- [ ] `PackLoader::load_as_vertical` returns `Vertical` for an installed pack id.
- [ ] All 5 tests in `pack_adapter_round_trip_test.rs` pass.
- [ ] `cargo clippy -p argo-vertical-packs --all-targets -- -D warnings` clean.
- [ ] Workspace clippy fast-fail clean (or boundary-noted with file:line if pre-existing).
- [ ] No `Vertical` field is left as `Default::default()` or `None` when the pack `worker.toml` carries the corresponding data — the adapter is **complete-as-possible**, not lossy.
- [ ] `cargo test -p argo-vertical-packs --test pack_adapter_round_trip_test` shows 5/5 green.

## May-touch

- `crates/argo-vertical-packs/src/adapter.rs` (NEW)
- `crates/argo-vertical-packs/src/loader.rs` (add `load_as_vertical`)
- `crates/argo-vertical-packs/src/lib.rs` (export adapter module)
- `crates/argo-vertical-packs/tests/pack_adapter_round_trip_test.rs` (NEW)
- `crates/argo-vertical-packs/Cargo.toml` (if a new dep on `argo-vertical` is needed)

**Out of may-touch**: any other crate, including `argo-vertical` itself. The adapter sits in `argo-vertical-packs` and depends on `argo-vertical` (one-way). No `packs/` or `verticals/` fixture data is added or removed by this brief.

## Parallel surfaces

- `rg 'pack.*vertical' crates/ --type rust` returns the existing `PackLoader` + `vertical_id` field references. No prior `pack_dir_to_vertical` or `load_as_vertical` symbol. Net-new.
- `rg 'WorkerManifest' crates/` confirms the `packs/` schema parser exists today (`crates/argo-vertical-packs/src/manifest.rs:147`). The adapter reads it; do not duplicate parsing.
- `argo_skill::pack_lint` reads `packs/` for lint purposes only — leave it alone.
- The audit report cite: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` §"`argo-vertical-packs`" row, F-2.

## Sensible deviations

- If `WorkerManifest` lacks a field that `Vertical` requires (e.g. `risk_class`), default to the safest value (`risk_class: RiskClass::Standard`) and record the choice in a top-of-file rustdoc comment. Do not invent fields in `worker.toml`.
- If `argo_vertical` does not expose `Vertical` publicly (private struct via load-only API), use the smallest re-export it does expose. The adapter target may be a `WorkflowDefinition` instead of `Vertical` — the contract is "can be loaded by the same runtime path."
- If round-tripping fixture pack `packs/project-coordinator` surfaces a `worker.toml` schema bug, fix the bug in the same PR (it's may-touch as `argo-vertical-packs` code) and call out the fix as a sensible deviation in the PR body.
- If `argo_vertical::Vertical` requires a workspace-id stamp for downstream lookups, accept a `workspace_id: WorkspaceId` argument on `load_as_vertical` rather than guessing — pass-through over imputation.
