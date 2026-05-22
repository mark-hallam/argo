# claude-config-loader-overhaul — scaffold argo-config + audit list (no callsite migration)

**Owner**: claude. **Depends on**: nothing. **Phase A consolidation.**

**Authored against**: origin/main b3949a73 on 2026-05-09 (rewrite v2 by conductor — original v1 bundled scaffold + workspace-wide migration of ~30 callsites into one brief; that's too big for one iteration. Splitting into scaffold-first, then per-area migration follow-ups).

## Why

Config loading today is **scattered across crates**:
- Env vars are read directly via `std::env::var("ARGO_*")` in ~30 sites across handlers
- `.skycot/*.toml` files are loaded ad-hoc in `argo-vertical`, `argo-cli/handlers/gateway`, `argo-marketplace`
- Default values live as inline `unwrap_or_else(|| "...".to_string())` in dozens of places

This brief produces the central scaffold + the audit list. **No callsite migrations.** The migrations land via per-area follow-up briefs.

## Approach (concrete)

1. **Verify crate location** — `ls crates/ | grep config`. If `argo-config` exists, extend it. If not, create it (workspace member add).
2. **NEW or extended `crates/argo-config/src/loader.rs`** (~300 LOC):
   - `pub struct ArgoConfig` — flat struct with every workspace-level config field. Each field has a doc-comment + a default.
   - `pub fn load() -> ArgoResult<ArgoConfig>` — single entry point. Layered precedence (highest wins): env var → `.skycot/argo.toml` → `~/.config/argo/argo.toml` → built-in defaults.
   - `pub fn load_from(path: &Path) -> ArgoResult<ArgoConfig>` — explicit-path variant for tests.
   - `impl ArgoConfig { pub fn from_env() -> ArgoResult<Self>; pub fn from_toml(s: &str) -> ArgoResult<Self>; pub fn merge(&mut self, other: Self); }`
3. **NEW `crates/argo-config/src/defaults.rs`** (~100 LOC) — every default value as a named const.
4. **NEW `crates/argo-config/src/lib.rs`** (or extend) — re-export `ArgoConfig`, `load`, `load_from`.
5. **NEW `scripts/check-no-direct-env-reads.sh`** (≤ 60 LOC) — CI heuristic that flags `std::env::var("ARGO_*")` outside `argo-config`. **Warning-only initially** (will flip to fail in the migration follow-up).
6. **NEW `docs/skycot/CONFIG_CALLSITE_AUDIT.md`** — checked-in audit listing every direct env-var/TOML callsite outside `argo-config` (path:line + env-var name + recommended ArgoConfig field). Becomes the input for the migration brief.
7. **Wire the audit script into CI as a warning-only step** in existing CI workflow.

**Explicitly out of scope for this brief**: any handler or crate-level migration. The scaffold lands; callsites stay as-is until the follow-up brief.

## Files you may touch

- `crates/argo-config/src/loader.rs` (NEW or extend, ≤ 350 LOC)
- `crates/argo-config/src/defaults.rs` (NEW, ≤ 200 LOC)
- `crates/argo-config/src/lib.rs` — narrowly: add `pub mod loader; pub mod defaults;` + re-exports
- `crates/argo-config/Cargo.toml` — narrowly: add `toml`, `serde` deps if not already present
- `Cargo.toml` (workspace) — narrowly: add `argo-config` as a member if creating the crate
- `scripts/check-no-direct-env-reads.sh` (NEW, ≤ 60 LOC)
- `docs/skycot/CONFIG_CALLSITE_AUDIT.md` (NEW, the audit report)
- `.github/workflows/ci.yml` — narrowly: 1 warning-only step invoking the new script

## Files you must NOT touch

- **Any callsite of `std::env::var("ARGO_*")` or `.skycot/argo.toml` outside `argo-config`** — migrations belong to the follow-up `claude-config-loader-migrate-callsites`.
- Any test file other than the new `loader_test` doctests in `argo-config`
- TUI key bindings
- `argo-types` (no new types)

## Acceptance criteria

- [ ] `argo_config::load()` returns a populated `ArgoConfig` for the test layered precedence
- [ ] `docs/skycot/CONFIG_CALLSITE_AUDIT.md` exists with one row per direct callsite
- [ ] `scripts/check-no-direct-env-reads.sh` runs and reports a count
- [ ] CI workflow has a new warning-only step
- [ ] `cargo build && cargo clippy -p argo-config --all-targets -- -D warnings && cargo test -p argo-config` clean (single-crate scope)
- [ ] No public API change in any other crate
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-config-loader-overhaul`
- PR title: `config: scaffold argo-config + ArgoConfig + callsite audit (claude-config-loader-overhaul)`

## Notes for the implementer

- The scaffold is the deliverable. The audit list (`CONFIG_CALLSITE_AUDIT.md`) is what the follow-up migration brief consumes.
- Layered precedence: env > project-local TOML > user-config TOML > built-in. Be explicit in the doc-comment.
- Per-tenant overrides are NOT in scope.
- Keep CI warning-only here so this brief lands without forcing migrations to land in the same iteration.

## Follow-up

Once this lands, the conductor will queue `claude-config-loader-migrate-callsites` for the mechanical migration of every callsite in the audit report.
