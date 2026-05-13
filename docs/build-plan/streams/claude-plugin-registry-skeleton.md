# claude-plugin-registry-skeleton — add `PluginRegistry` to `argo-plugin` for plugin discovery + lifecycle

**Owner**: claude. **Depends on**: nothing (precursor).
**Authored against**: origin/main `b77b7474` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-plugin-registry-skeleton`.

## Scope

Precursor brief for `mission-forge-stubforge-cleanup` (DEEP-DRIFT) which cites a non-existent `PluginRegistry` in `argo-plugin`. Today `argo-plugin` ships `PluginHost` (the per-plugin JSON-RPC subprocess runner — `crates/argo-plugin/src/host.rs`) but no central registry that:

- Discovers installed plugins (from a config-driven manifest or filesystem search)
- Spawns + caches `PluginHost` instances on first use
- Provides typed `registry.get_forge_plugin(name)` / `registry.get_executor_plugin(name)` accessors
- Handles graceful shutdown of all spawned plugin subprocesses on process exit

`mission-forge-stubforge-cleanup` and other future plugin-using briefs need this central registry.

Ship the skeleton:

1. **`PluginRegistry` struct** in `crates/argo-plugin/src/registry.rs` (≤350 LOC):
   - `pub fn new(config: PluginRegistryConfig) -> Self`
   - `pub async fn get_or_spawn(&self, plugin_id: &str) -> PluginResult<Arc<PluginHost>>` — spawns on first call, returns cached `Arc<PluginHost>` thereafter
   - `pub async fn shutdown_all(&self) -> PluginResult<()>` — graceful shutdown of every spawned plugin
   - `pub fn list_known(&self) -> Vec<&str>` — list plugin_ids from config (whether spawned or not)
   - Backing storage: `Mutex<HashMap<String, Arc<PluginHost>>>` (low-traffic; the contention isn't worth `DashMap` yet)

2. **`PluginRegistryConfig`** in `crates/argo-config/src/sections_context.rs` (since `RuntimeConfig` lives there):
   - `pub struct PluginRegistryConfig { pub plugins: Vec<PluginManifestEntry> }`
   - `pub struct PluginManifestEntry { pub id: String, pub binary_path: PathBuf, pub timeout_secs: u64 }`
   - Defaults: empty plugin list (operators opt in).

3. **No production wire-up** — that's a follow-up brief (`mission-plugin-registry-wire-agent-deps`). This brief ships only the registry primitive + config + tests; consuming it from `argo_agent_runner` or elsewhere is deferred.

**Not in scope**: dynamic plugin reloading (process restart only); plugin sandboxing beyond subprocess isolation (cgroup integration is `mission-cgroup-per-worker-resource-limits`'s territory); plugin marketplace integration.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `<NEW: crates/argo-plugin/src/registry.rs>` (≤350 LOC) — `PluginRegistry` + `PluginRegistryConfig`.
- `<NEW: crates/argo-plugin/src/registry_tests.rs>` (≤250 LOC) — 6 unit tests: empty-config registry; get-or-spawn caches; multiple-plugin isolation; shutdown_all stops every host; list_known reflects config; missing-plugin-id returns clear error.
- `crates/argo-plugin/src/lib.rs` — narrowly scoped: add ONE `pub mod registry;` line + `pub use registry::{PluginRegistry, PluginRegistryConfig, PluginManifestEntry};` + `#[cfg(test)] mod registry_tests;`.
- `crates/argo-config/src/sections_context.rs` — narrowly scoped: add `PluginRegistryConfig` (and `PluginManifestEntry`) + ONE field on `RuntimeConfig`. ≤30 modified lines.
- `crates/argo-config/src/lib.rs` — narrowly scoped: re-export `PluginRegistryConfig` from `sections_context`. ≤2 modified lines.

## Files you must NOT touch

- `crates/argo-plugin/src/host.rs` — `PluginHost` primitive is locked.
- `crates/argo-plugin/src/protocol.rs` — JSON-RPC envelope is locked.
- `crates/argo-forge/**` — `StubForge` consumption is the follow-up brief (`mission-forge-stubforge-cleanup` rewrite).
- `crates/argo-agent-runner/**` — wire-up to AgentDeps is the follow-up brief.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `PluginRegistry::new(config)` constructs an empty registry.
- [ ] `get_or_spawn("plugin-id")` returns a freshly-spawned `Arc<PluginHost>` on first call; same `Arc` on subsequent calls (verify via `Arc::ptr_eq`).
- [ ] `get_or_spawn` for an unknown plugin id returns a typed error (e.g. `PluginError::UnknownPlugin(id)`).
- [ ] `shutdown_all` calls `host.shutdown()` on every cached host and returns once all complete.
- [ ] `list_known` returns config-declared ids regardless of whether they've been spawned yet.
- [ ] Per-plugin isolation: 2 distinct plugins get 2 distinct `Arc<PluginHost>` instances.
- [ ] All 6 unit tests pass (mock `PluginHost` or use a test-fixture binary if available).
- [ ] `cargo clippy -p argo-plugin -p argo-config --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-plugin` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-plugin-registry-skeleton`
- PR title: `plugin: PluginRegistry for plugin discovery + lifecycle (claude-plugin-registry-skeleton)`

## Notes for the implementer

- The host-spawn flow already exists in `crates/argo-plugin/src/host.rs::PluginHost::spawn`. Registry just wraps the spawn + caching.
- For tests, you have two options: (a) mock the `PluginHost` via a test trait that registry holds (slightly bigger refactor), (b) use the existing `argo-plugin/tests/` test-fixture plugin binary if one exists. Pick whichever lands faster; document the choice.
- `PluginError::UnknownPlugin(id)` may need to be added to `crates/argo-plugin/src/error.rs` — that's in-scope as a tiny additive change.
- Unblocks `mission-forge-stubforge-cleanup` (DEEP-DRIFT) once landed.
- The follow-up wire-up brief (`mission-plugin-registry-wire-agent-deps`) is NOT authored here — author it after this lands so the may-touch reflects whatever surface this brief actually shipped.
