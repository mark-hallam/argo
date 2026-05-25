# eval-argo-config-crate-surface

Date: 2026-05-24
Brief: `eval-argo-config-crate-surface`
Verdict: PASS WITH FOLLOW-UP

## Scope

Audited the public surface exported from `crates/argo-config/src/lib.rs`:

- Loader entry points: `load`, `load_from`, `ArgoConfig::from_toml`, `ArgoConfig::from_toml_with_overrides`, and `ArgoConfig::merge`.
- Public modules and re-exports: `defaults`, `loader`, `provider`, `source_tracking`, `store`, `tui`, `worker_pool`, provider entries, sandbox config, store config, TUI config, and worker-pool config.
- Mission-relevant section schemas and defaults: gateway, auth, telemetry, SIEM, code executor, planner, and runtime worker-resource cage.
- Source display annotations through `source_tracking::ConfigSourceMap`.

## Evidence Added

Added `crates/argo-config/tests/surface_contract.rs` as an external integration test that compiles only through public `argo_config` APIs. It pins:

- Public type/default reachability for the mission-relevant config sections.
- TOML parsing for gateway, auth, telemetry cardinality caps, SIEM, code executor, planner bias, and worker-resource cage.
- Layered loader precedence for supported file/env fields with temp user/project fixtures.
- Source tracking labels for env, user file, project file, and default values.
- Explicit override merge behavior through `from_toml_with_overrides` and `merge`.

## Finding

`ArgoConfig::from_toml` parses all audited mission sections, but the layered loader merge mask is narrower than the public schema. `loader_overrides::ArgoConfigOverrides` and `ArgoConfig::merge` currently cover auth/core/tenant/telemetry/error-reporting/gateway/channel-webinbox fields, but do not carry masks for several public sections such as `[siem]`, `[code_executor]`, `[planner]`, or `[runtime.worker_resource_cage]`. That means a direct `ArgoConfig::from_toml` call can parse those sections, while `load_from` can fail to apply them from user/project layers because the merge layer treats unmasked fields as unset.

Follow-up brief name: `fix-argo-config-layered-loader-full-section-overrides`.

## Verification

- `bash scripts/check-brief-freshness.sh eval-argo-config-crate-surface` reported only authored-SHA distance drift; current crate scope still matched the brief.
- `rustfmt --check crates/argo-config/tests/surface_contract.rs`
- `cargo test -p argo-config --tests`
- `cargo clippy -p argo-config --all-targets -- -D warnings`

Boundary note: `cargo fmt -p argo-config -- --check` reports pre-existing rustfmt drift in `crates/argo-config/src/cred/db_key.rs`, `crates/argo-config/src/cred/db_key_tests.rs`, `crates/argo-config/src/cred/io.rs`, `crates/argo-config/src/cred.rs`, and `crates/argo-config/src/lib.rs`, all outside this brief's may-touch list.
