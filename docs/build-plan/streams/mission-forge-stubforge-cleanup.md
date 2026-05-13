# Mission load-bearing wire — Forge trait cleanup: replace `StubForge` OR rename `forge_optimize` as canonical

**Owner**: claude. **Depends on**: `argo-forge::Forge` trait + `StubForge` impl SHIPPED (verified at `crates/argo-forge/src/forge.rs:20,33`); `argo-forge::run::{ForgeOptions, ForgeRun, ForgeStatus}` SHIPPED; gateway `forge_optimize_response` HTTP handler SHIPPED (at `crates/argo-cli/src/handlers/gateway/routes_post_forge.rs:38`).
**Authored against**: origin/main `939e3bec` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-forge-stubforge-cleanup`.

**DEEP-DRIFT — CONDUCTOR REWRITE NEEDED (precursor missing)**: `argo-plugin` has no `PluginRegistry` / call-by-id surface (only single-shot `PluginHost::start(binary_path, args)`); `[plugin].auto_load` is a config field but no machinery launches plugins from it. Brief explicitly says "if no clean call-by-id surface, BLOCK and request a precursor brief rather than expand scope." Required precursor: `argo-plugin gains a PluginRegistry` that owns one `PluginHost` per id (probably loaded from `[plugin].auto_load`), with a `call(id, method, params)` surface — then this brief can wire `LocalForge → registry.call(worker_plugin_id, "optimize", opts)` cleanly. ALSO `crates/argo-config/src/forge.rs` does NOT exist; forge config lives in `crates/argo-config/src/sections_routing.rs` today. ALSO `crates/argo-cli/src/handlers/forge_run.rs` does not exist; forge handlers are `forge_optimize.rs`, `forge_admin.rs`, `forge_daemon.rs`, etc. The brief is blocked on the precursor regardless of these path fixes.

> SOURCE: MISSION_BACKLOG.md Section 4/Section 7 Claim 4 #84 + #262 — "Replace `StubForge` trait OR delete trait + rename `forge_optimize` as canonical." Today the Forge surface has two awkward shapes coexisting: (1) a `Forge` trait with a single `optimize(...)` method whose only impl is `StubForge` (which always errors); (2) a gateway HTTP handler called `forge_optimize_response` that bypasses the trait entirely (no Python worker plugin shipped). Mission promise: ONE canonical entry point so downstream callers stop guessing.

## Scope

Pick path **(B) Rename** (less destructive than deleting the trait). Specifically:

1. **Rename** `StubForge` → `LocalForge` and document it as the canonical no-Python-worker fallback (NOT a stub). Keep the same surface — it still returns `ArgoError::Provider` with the "configure forge.worker" message when no plugin is registered. The rename signals to readers it's the production no-plugin path.

2. **Add a `worker_plugin_id: Option<String>`** field to `LocalForge`. When `Some(id)`, the impl dispatches to a registered worker plugin via existing argo-plugin machinery (verify: `grep -rn 'plugin_dispatch\|PluginRegistry' crates/argo-runtime/src/ crates/argo-core/src/ | head -5` in first 5min). When `None`, behaves identical to today's StubForge.

3. **`forge_optimize_response` HTTP handler** (at `routes_post_forge.rs:38`): refactor to call `Forge::optimize` through a `&dyn Forge` parameter instead of doing its own ad-hoc logic. The gateway boot constructs `LocalForge::new(worker_plugin_id)` and threads it as application state. NO change to HTTP request/response shape.

4. **Compatibility shim**: `pub type StubForge = LocalForge;` + `#[deprecated(note="use LocalForge")]`. Existing callers (CLI handlers + tests) keep compiling. Document removal target ("after one minor release").

5. **CLI surface**: `argo forge run <skill>` already exists (verify). Narrow change: now constructs `LocalForge::new(config.forge.worker_plugin_id)` from `argo.toml` instead of `StubForge::new()`. Empty config → unchanged "configure forge.worker" error.

This is NOT in scope: shipping the actual Python DSPy/GEPA worker plugin (that's `mission-forge-dspy-gepa-worker` — separate brief if not authored); changing `ForgeOptions` / `ForgeRun` shapes; persistence migration (`forge_run` table stable).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-forge/src/forge.rs` — narrowly scoped: rename `StubForge` → `LocalForge`, ADD `worker_plugin_id: Option<String>` field + constructor variant, ADD plugin-dispatch branch in `optimize()`. File currently ~140 LOC; stay under 250 (well under ceiling).
- `crates/argo-forge/src/lib.rs` — narrowly scoped: re-export rename + ADD `#[deprecated]` `StubForge` type alias.
- `crates/argo-forge/src/forge_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests: id-stable-as-local; no-plugin-returns-provider-error; with-plugin-dispatches-to-registry; plugin-error-bubbles; forge-options-roundtrip; deprecated-alias-resolves.
- `crates/argo-cli/src/handlers/gateway/routes_post_forge.rs` — narrowly scoped: replace ad-hoc body with `forge.optimize(opts).await?` and a `&dyn Forge` parameter. ≤ 30 LOC delta.
- `crates/argo-cli/src/handlers/forge_run.rs` (verify path) OR existing `handlers/forge.rs` — narrowly scoped: instantiate `LocalForge::new(...)` reading the new `argo.toml` knob.
- `crates/argo-config/src/forge.rs` (verify path) OR sibling — narrowly scoped: ADD `worker_plugin_id: Option<String>` to the `[forge]` config section. Default = None (back-compat). ≤ 20 LOC delta.
- `crates/argo-cli/src/handlers/gateway/mod.rs` (verify) — narrowly scoped: construct + inject `LocalForge` at boot for the route handler.

## Files you must NOT touch

- `crates/argo-forge/src/run.rs` — ForgeOptions / ForgeRun / ForgeStatus / ForgeEvent are stable; reuse only
- `crates/argo-forge/src/finalize.rs` (if present) — reuse only
- Any persistence (`crates/argo-store/`) — no schema change
- Any non-forge/non-cli/non-config crate

## Acceptance criteria

- [ ] `LocalForge` exists with `worker_plugin_id: Option<String>` field
- [ ] `StubForge` is a `#[deprecated]` type alias for `LocalForge`
- [ ] No-plugin path returns same `ArgoError::Provider("configure forge.worker...")` as today
- [ ] With-plugin path dispatches via existing argo-plugin machinery
- [ ] `forge_optimize_response` gateway handler now calls `Forge::optimize` via trait, no ad-hoc forge logic
- [ ] `argo.toml` `[forge].worker_plugin_id` parses correctly; empty/missing → None
- [ ] 6 unit tests cover trait surface + plugin dispatch + back-compat
- [ ] Existing argo-cli tests that reference `StubForge` still pass byte-identical (alias resolves)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-forge -p argo-cli -p argo-config` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-forge-stubforge-cleanup`
- PR title: `forge: rename StubForge→LocalForge + plugin-dispatch path + canonical trait entry (mission-forge-stubforge-cleanup)`

## Notes for the implementer

- The deprecation alias is **load-bearing**: many existing tests/handlers import `StubForge` directly. Breaking them would explode the diff. The alias keeps them green while signalling the new canonical name.
- For plugin dispatch: verify the argo-plugin / argo-runtime plugin-registration pattern (search `PluginRegistry`, `register_plugin`, or `argo_plugin::registry`) — if it doesn't have a clean call-by-id surface, BLOCK and request a precursor brief rather than expand scope.
- The actual Python DSPy/GEPA worker is a **separate** brief — this one only opens the seam.
- Don't try to delete the trait. The trait+plugin-id pattern is the right v1 shape.
- `forge_optimize_response` may have additional logic (auth, request validation) — preserve all of that; only swap the "compute the run" core.
