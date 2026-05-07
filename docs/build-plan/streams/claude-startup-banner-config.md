# claude-startup-banner-config — make gateway startup banner configurable

**Owner**: claude. **Depends on**: #935 (start_banner module landed). **Phase A polish.**

## Why

The gateway startup banner (printed by `argo gateway start` to stdout once the listener is bound) is **hard-coded** in `crates/argo-cli/src/handlers/gateway/start_banner.rs` (introduced by #935 from Tick 32 work). Operators running Argo in production want to:

- Customise the banner (corporate branding, deployment ID, region, environment tag)
- Suppress it entirely (`--quiet` mode, structured-log-only deployments)
- Print extra fields (build SHA, deploy timestamp, feature flags active)

Today none of this is possible without forking. This brief makes the banner config-driven without changing the default output.

## Approach (concrete)

**Extend the existing module.** Default behaviour preserved (default banner printed verbatim if no config set). Adds two layers of configuration: env var overrides and a `[startup_banner]` section in `argo.toml` (or whichever config file the loader uses, courtesy of `claude-config-loader-overhaul` if landed).

1. **Modify `crates/argo-cli/src/handlers/gateway/start_banner.rs`** (currently ~?LOC; verify):
   - Introduce `pub struct BannerConfig { enabled: bool, header: Option<String>, footer: Option<String>, extra_fields: Vec<(String, String)>, mode: BannerMode }`.
   - `pub enum BannerMode { Default, Custom, Quiet }`.
   - `pub fn render(config: &BannerConfig) -> String` — the canonical render path.
   - `pub fn render_default() -> String` — wraps `render(&BannerConfig::default())`. Existing callers point here, byte-identical output.
2. **NEW `crates/argo-cli/src/handlers/gateway/start_banner_config.rs`** (~150 LOC):
   - `pub fn load_from_env() -> BannerConfig` — reads `ARGO_BANNER_ENABLED`, `ARGO_BANNER_HEADER`, `ARGO_BANNER_FOOTER`, `ARGO_BANNER_QUIET`.
   - `pub fn load_from_argo_config(cfg: &ArgoConfig) -> BannerConfig` — reads `[startup_banner]` section.
   - `pub fn merge(base: BannerConfig, override_: BannerConfig) -> BannerConfig` — env wins over file wins over default.
3. **Modify `crates/argo-cli/src/handlers/gateway/start.rs`** — narrowly: replace the hard-coded `print!(banner)` with `print!(start_banner::render(&start_banner_config::load_layered()))`.
4. **CLI flag**: extend `argo gateway start` with `--quiet` (mapped to `BannerMode::Quiet`). Single clap field addition.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/start_banner.rs` — narrow extension (split if it exceeds 500 LOC)
- `crates/argo-cli/src/handlers/gateway/start_banner_config.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/handlers/gateway/start.rs` — narrowly: 1-line render-call swap
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: `mod start_banner_config;`
- `crates/argo-cli/src/cli.rs` (or per-family clap module) — narrowly: add `--quiet` field
- `crates/argo-cli/tests/banner_smoke.rs` (NEW, ≤ 200 LOC) — 5 tests:
  - Default banner output unchanged byte-for-byte
  - `ARGO_BANNER_QUIET=1` → empty output
  - Custom header via env var → reflected in output
  - `[startup_banner]` TOML override → reflected in output
  - `--quiet` flag → empty output (overrides env)

## Files you must NOT touch

- Any non-banner gateway start logic
- Tracing / logging — banner goes to stdout, not the log subsystem
- `argo-config` core (depends on it via interface, doesn't extend it here)

## Acceptance criteria

- [ ] Default `argo gateway start` output is byte-identical to today
- [ ] `ARGO_BANNER_QUIET=1` suppresses the banner
- [ ] `--quiet` flag suppresses the banner (CLI > env precedence)
- [ ] `[startup_banner]` TOML section overrides individual fields
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-startup-banner-config`
- PR title: `gateway: configurable startup banner (claude-startup-banner-config)`

## Notes for the implementer

- The default-output byte-identical guarantee is the trickiest part — capture the current output once (run `argo gateway start --port 0 2>/dev/null` and snapshot stdout), then assert against it in the smoke test.
- Precedence: CLI flag > env var > TOML > built-in default. Document this in the new file's header comment.
- Don't add a `--banner` CLI option for arbitrary text overrides — that's flag-bloat. Env var + TOML is enough.
- Extra fields (build SHA, deploy timestamp) should slot in from `ArgoConfig` if claude-config-loader-overhaul has landed — otherwise leave the hooks for later.
- This brief deliberately does NOT touch the banner's content/wording. That's a product call, not a config call.
