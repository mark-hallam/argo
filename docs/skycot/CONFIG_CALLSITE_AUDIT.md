# Config callsite audit

Snapshot of every direct `std::env::var("ARGO_*")` read in production
Rust source (`crates/`, `mcp-servers/`) at the time the
`claude-config-loader-overhaul` scaffold landed. Test files
(`tests/`, `*_tests.rs`, `tests.rs`) and the `crates/argo-config/`
crate itself are exempt — tests legitimately manipulate env vars to
exercise the loader, and `argo-config` is the one place direct reads
belong.

This file is the **input** for the follow-up brief
`claude-config-loader-migrate-callsites`. Each row is migrated to
read through `argo_config::load()` (extending `ArgoConfig` with a
new field where the env var is not yet bound). Once the audit
reaches zero rows, the warning-only ratchet in
`scripts/check-no-direct-env-reads.sh` flips to a failing CI gate.

Methodology: `scripts/check-no-direct-env-reads.sh` runs the
identical exclusion glob and reports the same callsites.

## Summary

- 21 production-Rust files read `ARGO_*` env vars directly.
- 38 distinct `std::env::var("ARGO_…")` call expressions across
  those files (some files have multiple reads).
- 25 distinct env-var keys.
- Concentrations: `argo-cli/src/handlers/` (15 files,
  ~30 reads) — the gateway routes, worker action handlers, and the
  `budget_watch` daemon are the largest clusters. `argo-tool`,
  `argo-tui`, `argo-store`, `argo-identity`, and the voice MCP each
  hold one or two.

## Callsites

Path:line columns identify the exact `std::env::var` expression. The
"Recommended `ArgoConfig` field" column proposes where the value
belongs in the flat config struct; fields prefixed `(new)` need to be
added by the migration brief because the scaffold did not bind them
upfront (the scaffold prioritized fields whose defaults were already
referenced by hot-path callsites — gateway port/url, duty scheduler,
handoff sweep — over the long tail of channel-specific knobs).

### `argo-cli` — handlers

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-cli/src/handlers/budget_watch.rs:241` | `ARGO_CHANNEL_HMAC_SECRET` | (new) `channels.hmac_secret` |
| `crates/argo-cli/src/handlers/budget_watch.rs:254` | `ARGO_BUDGET_ALERT_OPERATOR_ID` | (new) `budget.alert_operator_id` |
| `crates/argo-cli/src/handlers/budget_watch.rs:463` | `ARGO_BUDGET_WATCH_SECS` | (new) `budget.watch_interval_secs` |
| `crates/argo-cli/src/handlers/gateway/boot.rs:189` | `ARGO_DUTY_SCHEDULER_SECS` | `gateway.duty_scheduler_secs` (bound) |
| `crates/argo-cli/src/handlers/gateway/routes_auth.rs:30` | `ARGO_GATEWAY_AUTH_BYPASS` | (new) `gateway.auth_bypass` |
| `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs:108` | `ARGO_WEBINBOX_OPERATOR_ID` | (new) `channels.webinbox.operator_id` |
| `crates/argo-cli/src/handlers/gateway/routes_channels_whatsapp.rs:114` | `ARGO_TWILIO_WHATSAPP_DECISION_URL` | (new) `channels.whatsapp.decision_callback_base_url` |
| `crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs:333` | `ARGO_WEBINBOX_TENANT_ID` | (new) `channels.webinbox.tenant_id` |
| `crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs:333` | `ARGO_DEFAULT_TENANT` | `tenant.default_id` (bound) |
| `crates/argo-cli/src/handlers/gateway/routes_voice.rs:230` | `ARGO_TWILIO_VOICE_DECISION_URL` | (new) `channels.voice.decision_callback_base_url` |
| `crates/argo-cli/src/handlers/gateway/sweepers.rs:32` | `ARGO_HANDOFF_SWEEP_SECS` | `gateway.handoff_sweep_secs` (bound) |
| `crates/argo-cli/src/handlers/session.rs:383` | `ARGO_DEFAULT_TENANT` | `tenant.default_id` (bound) |
| `crates/argo-cli/src/handlers/voice.rs:184` | `ARGO_WHISPER_MODEL` | (new) `voice.whisper_model` |
| `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:127` | `ARGO_CODE_EXEC_TIMEOUT_S` | (new) `executor.code.timeout_s` |
| `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:132` | `ARGO_CODE_EXEC_MEM_MIB` | (new) `executor.code.memory_mib` |
| `crates/argo-cli/src/handlers/worker_actions_notify.rs:154` | `ARGO_NOTIFY_DEFAULT_EMAIL` | (new) `notify.default_email` |
| `crates/argo-cli/src/handlers/worker_actions_notify.rs:328` | `ARGO_WEBINBOX_PATH` | (new) `channels.webinbox.path` |
| `crates/argo-cli/src/handlers/worker_create.rs:481` | `ARGO_OPERATOR` | (new) `core.operator` |
| `crates/argo-cli/src/handlers/workspace_marketplace.rs:313` | `ARGO_OCI_TOKEN` | (new) `marketplace.oci_token` |

### `argo-identity`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-identity/src/circuit.rs:51` | `ARGO_CIRCUIT_THRESHOLD` | (new) `identity.circuit_threshold` |

### `argo-store`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-store/src/anomaly.rs:22` | `ARGO_ANOMALY_Z_THRESHOLD` | (new) `store.anomaly_z_threshold` |
| `crates/argo-store/src/spend_anomaly.rs:57` | `ARGO_SPEND_ANOMALY_Z` | (new) `store.spend_anomaly_z` |

### `argo-tool`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-tool/src/tools/web/mod.rs:293` | `ARGO_LIVE_WEB` | (new) `tools.web.live` |
| `crates/argo-tool/src/tools/web/mod.rs:331` | `ARGO_LIVE_WEB` | (new) `tools.web.live` |
| `crates/argo-tool/src/tools/web/search.rs:59` | `ARGO_DDG_HTML_URL` | (new) `tools.web.ddg_html_url` |
| `crates/argo-tool/src/tools/web/search.rs:82` | `ARGO_TAVILY_URL` | (new) `tools.web.tavily_url` |
| `crates/argo-tool/src/tools/web/search.rs:134` | `ARGO_BRAVE_URL` | (new) `tools.web.brave_url` |

### `argo-tui`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-tui/src/overlays_h_stack.rs:58` | `ARGO_GATEWAY_URL` | `gateway.url` (bound) |
| `crates/argo-tui/src/overlays_h_stack.rs:157` | `ARGO_GATEWAY_URL` | `gateway.url` (bound) |

### MCP servers

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `mcp-servers/voice/src/whisper_native.rs:31` | `ARGO_WHISPER_MODEL` | (new) `voice.whisper_model` (or kept as MCP env — out-of-process binary) |

## Ad-hoc TOML readers (advisory)

The brief flagged `.skycot/*.toml` and `argo.toml` ad-hoc loaders as
the second wedge of scattered config. The scaffold does not migrate
these; they're listed so the follow-up brief can decide whether to
absorb them into `argo_config::load()` or leave them as
domain-specific files (e.g. auto-approval policy and notify rules
have their own schemas and were never `argo.toml` keys).

| Path | What it loads |
| --- | --- |
| `crates/argo-cli/src/handlers/agents.rs` | `argo.toml` (read + write `[agents.*]`) |
| `crates/argo-cli/src/handlers/costs.rs` | `argo.toml` (read + write `[budget]`) |
| `crates/argo-cli/src/handlers/doctor_fix.rs` | `argo.toml` (via `ArgoConfig::load`) + `.skycot/auto_approval.toml` |
| `crates/argo-cli/src/handlers/doctor_h_stack.rs` | `.skycot/auto_approval.toml` |
| `crates/argo-cli/src/handlers/gateway/mod.rs` | `argo.toml` (via `ArgoConfig::load`) |
| `crates/argo-cli/src/handlers/gateway/routes_meta.rs` | `argo.toml` (via `ArgoConfig::load`) |
| `crates/argo-cli/src/handlers/onboard.rs` | `.argo/onboarding.toml` |
| `crates/argo-cli/src/handlers/onboard_stages.rs` | `argo.toml` (read + write) |
| `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` | `.skycot/auto_approval.toml` |
| `crates/argo-cli/src/handlers/worker_actions_notify.rs` | `.skycot/notify.toml` |
| `crates/argo-cli/src/handlers/workspace.rs` | `argo.toml` (read + write) |
| `crates/argo-identity/src/classification.rs` | `.argo/classification.toml` |

The `ArgoConfig::load(&cwd.join("argo.toml"))` callers (gateway/mod,
gateway/routes_meta, doctor_fix, workspace, costs) are the simplest
migration: swap in `argo_config::load()` so the layered loader picks
up env-var overrides for free.

## Bound vs unbound

The scaffold's `ArgoConfig` (see `crates/argo-config/src/sections.rs`
and `crates/argo-config/src/loader.rs`) binds these env vars in
`from_env()` upfront so the four hot-path callsites in `gateway/`
and `tui/` migrate as a one-liner once the follow-up runs:

- `ARGO_HOME` → `core.home`
- `ARGO_PROFILE` → `core.profile`
- `ARGO_DEFAULT_TENANT` → `tenant.default_id`
- `ARGO_TELEMETRY_ENABLED` → `telemetry.enabled`
- `ARGO_TELEMETRY_EXPORTER` → `telemetry.exporter`
- `ARGO_TELEMETRY_ENDPOINT` → `telemetry.endpoint`
- `ARGO_GATEWAY_HOST` → `gateway.host`
- `ARGO_GATEWAY_PORT` → `gateway.port`
- `ARGO_GATEWAY_URL` → `gateway.url`
- `ARGO_DUTY_SCHEDULER_SECS` → `gateway.duty_scheduler_secs`
- `ARGO_HANDOFF_SWEEP_SECS` → `gateway.handoff_sweep_secs`

Every other env-var key in the audit needs a new `ArgoConfig` field
added in the migration brief.
