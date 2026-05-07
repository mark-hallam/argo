# claude-config-loader-overhaul — centralize config loading into argo-config

**Owner**: claude. **Depends on**: nothing. **Phase A consolidation.**

## Why

Config loading today is **scattered across crates**:
- Env vars are read directly via `std::env::var("ARGO_*")` in ~30 sites across handlers
- `.skycot/*.toml` files are loaded ad-hoc in `argo-vertical`, `argo-cli/handlers/gateway`, `argo-marketplace`
- Default values live as inline `unwrap_or_else(|| "...".to_string())` in dozens of places
- The same env var (e.g. `ARGO_GATEWAY_PORT`) is parsed in 3+ places with different defaults and parse-error handling

Result: changing a default requires hunting every callsite, missing one means the new default applies inconsistently. Telemetry can't enumerate "every config option Argo respects" because there is no single source.

## Approach (concrete)

**Centralize into `argo-config`.** Existing crate location is uncertain — verify with `ls crates/ | grep config`. If the crate exists, extend it. If not, create it. The brief assumes extend.

1. **NEW or extended `crates/argo-config/src/loader.rs`** (~300 LOC):
   - `pub struct ArgoConfig` — flat struct with every workspace-level config field (gateway port, store URL, default tenant, default provider, marketplace registry URL, etc.). Each field has a doc-comment + a default.
   - `pub fn load() -> ArgoResult<ArgoConfig>` — single entry point. Layered precedence (highest wins): env var → `.skycot/argo.toml` → `~/.config/argo/argo.toml` → built-in defaults.
   - `pub fn load_from(path: &Path) -> ArgoResult<ArgoConfig>` — explicit-path variant for tests.
   - `impl ArgoConfig { pub fn from_env() -> ArgoResult<Self>; pub fn from_toml(s: &str) -> ArgoResult<Self>; pub fn merge(&mut self, other: Self); }`
2. **NEW `crates/argo-config/src/lib.rs`** (or extend) — re-export `ArgoConfig`, `load`, `load_from`.
3. **NEW `crates/argo-config/src/defaults.rs`** (~100 LOC) — every default value as a named const. (Lets handlers reference `argo_config::defaults::GATEWAY_PORT` instead of inline `8080`.)
4. **Migrate callsites**: in each handler / crate that currently reads env vars or TOML directly, replace with a single `ArgoConfig::load()?` at the entry point + thread the resolved values down. Narrow surgical edits — handlers receive `&ArgoConfig` (or relevant slice) instead of calling `std::env::var` themselves.

## Files you may touch

- `crates/argo-config/src/loader.rs` (NEW or extend, ≤ 350 LOC)
- `crates/argo-config/src/defaults.rs` (NEW, ≤ 200 LOC)
- `crates/argo-config/src/lib.rs` — narrowly: add `pub mod loader;` `pub mod defaults;` + re-exports
- `crates/argo-config/Cargo.toml` — narrowly: add `toml`, `serde` deps if not already present
- `crates/argo-cli/src/handlers/gateway/start.rs` — narrowly: switch to `ArgoConfig::load()`
- `crates/argo-cli/src/main.rs` (or wherever the top-level entry is) — narrowly: load config once, thread down
- Other handler files that currently read env vars — narrowly: accept `&ArgoConfig` parameter
- `scripts/check-no-direct-env-reads.sh` (NEW, ≤ 60 LOC) — CI heuristic that flags new `std::env::var("ARGO_*")` outside `argo-config`

## Files you must NOT touch

- Any test file other than the new `loader_test` doctests
- TUI key bindings (TUI has its own bindings file, not config)
- `argo-types` (no new types added there)

## Acceptance criteria

- [ ] `argo_config::load()` returns a populated `ArgoConfig` for the test layered precedence
- [ ] No direct `std::env::var("ARGO_*")` reads outside `argo-config` (verified by CI script)
- [ ] No direct `.skycot/argo.toml` reads outside `argo-config`
- [ ] Default-value behaviour preserved (sweep gateway integration tests confirm)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-config-loader-overhaul`
- PR title: `config: centralize config loading into argo-config (claude-config-loader-overhaul)`

## Notes for the implementer

- The migration is the long pole — budget ~70% of effort there. The new loader itself is straightforward.
- Layered precedence: env > project-local TOML > user-config TOML > built-in. Be explicit in the doc-comment.
- Where a callsite reads an env var that doesn't fit cleanly into the flat `ArgoConfig`, leave it for now and file a follow-up brief; do NOT shoehorn.
- Per-tenant overrides are NOT in scope — that's a separate brief.
- The CI script can be a simple `rg 'std::env::var\("ARGO_' --type rust --files-without-match` excluding `crates/argo-config/`.
