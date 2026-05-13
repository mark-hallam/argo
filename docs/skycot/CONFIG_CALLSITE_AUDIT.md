# Config callsite audit

Snapshot of every direct `std::env::var("ARGO_*")` read in production
Rust source (`crates/`, `mcp-servers/`). Test files (`tests/`,
`*_tests.rs`, `tests.rs`) and the `crates/argo-config/` crate itself
are exempt — tests legitimately manipulate env vars to exercise the
loader, and `argo-config` is the one place direct reads belong.

This file is the **input** for the follow-up brief
`claude-config-loader-migrate-callsites-v2`. Each row is migrated to
read through `argo_config::load()` (extending `ArgoConfig` with new
fields where the env var is not yet bound). Once the audit reaches
zero rows, the warning-only ratchet in
`scripts/check-no-direct-env-reads.sh` flips to a failing CI gate.

Methodology: `scripts/check-no-direct-env-reads.sh` runs the
identical exclusion glob and reports the same callsites.

## Summary

- **44 production-Rust files** read `ARGO_*` env vars directly
  (matches `bash scripts/check-no-direct-env-reads.sh` file count).
- **64 distinct `std::env::var("ARGO_…")` call expressions** across
  those files (some files have multiple reads; e.g.
  `gateway/rate_limit_ip.rs` reads four IP-rate-limit keys, and
  `security/pii_detect.rs` reads three PII flags).
- **52 distinct env-var keys**.
- **Concentrations**: `argo-cli/src/handlers/gateway/` (16 files, 28
  reads) — JWT pubkey loading, IP+tenant rate limiters, security
  headers, the duty/sweep/approval-expiry/shutdown sweepers, the
  channel-specific orchestrator routes, and CU screenshot retention.
  `argo-cli/src/handlers/` outside `gateway/` adds another 11 files
  (budget watch, voice, worker actions, mcp health, self-update,
  workspace marketplace). `argo-store` holds 4 files (anomaly,
  spend_anomaly, batched_writer, worker_archive). `argo-tool`,
  `argo-tui`, `argo-security`, `argo-channels`, `argo-genesis`,
  `argo-identity`, `argo-executor-document`, `argo-mcp-image-gen`,
  `argo-test-cassette`, and the voice MCP each hold one or two.

## Callsites

Path:line columns identify the exact `std::env::var` expression. The
"Recommended `ArgoConfig` field" column proposes where the value
belongs in the flat config struct; fields prefixed `(new)` need to be
added by the migration brief because the scaffold did not bind them
upfront (the scaffold prioritized fields whose defaults were already
referenced by hot-path callsites — gateway port/url, duty scheduler,
handoff sweep, default tenant — over the long tail of channel-,
security-, store- and tools-specific knobs).

Where the existing scaffold has no obvious home, the recommendation
names a **`NEW: sections_<cluster>.rs`** target — the cluster file
the migration brief will add alongside the existing `sections_*.rs`
siblings. See "Recommended cluster sections" below for the full
proposed split.

### `argo-channels`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-channels/src/fallback.rs:204` | `ARGO_DELIVERY_REPORTS_PATH` | (new) `channels.delivery_reports_path` → extend `sections_channels.rs` |

### `argo-cli` — handlers (non-gateway)

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-cli/src/handlers/budget_watch.rs:211` | `ARGO_CHANNEL_HMAC_SECRET` | (new) `channels.hmac_secret` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/budget_watch.rs:224` | `ARGO_BUDGET_ALERT_OPERATOR_ID` | (new) `budget.alert_operator_id` → extend `sections_budget.rs` |
| `crates/argo-cli/src/handlers/budget_watch.rs:260` | `ARGO_BUDGET_WATCH_SECS` | (new) `budget.watch_interval_secs` → extend `sections_budget.rs` |
| `crates/argo-cli/src/handlers/handoff_stats.rs:304` | `ARGO_DEFAULT_TENANT` | `tenant.default_id` (bound) |
| `crates/argo-cli/src/handlers/incident_bundle.rs:93` | `ARGO_HOME` | `core.home` (bound) |
| `crates/argo-cli/src/handlers/mcp_health.rs:283` | `ARGO_MCP_HEALTH_CHECK` | (new) `tools.mcp.health_check_enabled` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-cli/src/handlers/self_update.rs:369` | `ARGO_HOME` | `core.home` (bound) |
| `crates/argo-cli/src/handlers/session_orchestrator.rs:147` | `ARGO_DEFAULT_TENANT` | `tenant.default_id` (bound) |
| `crates/argo-cli/src/handlers/voice.rs:184` | `ARGO_WHISPER_MODEL` | (new) `channels.voice.whisper_model` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:127` | `ARGO_CODE_EXEC_TIMEOUT_S` | (new) `tools.executor.code.timeout_s` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-cli/src/handlers/worker_actions_executor_code.rs:132` | `ARGO_CODE_EXEC_MEM_MIB` | (new) `tools.executor.code.memory_mib` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs:200` | `ARGO_MCP_REGISTRY_BACKED` | (new) `tools.mcp.registry_backed` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-cli/src/handlers/worker_actions_notify.rs:186` | `ARGO_NOTIFY_DEFAULT_EMAIL` | (new) `channels.notify.default_email` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/worker_actions_notify.rs:419` | `ARGO_WEBINBOX_PATH` | (new) `channels.webinbox.path` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/worker_create_store.rs:266` | `ARGO_OPERATOR` | (new) `core.operator` → extend `sections_core.rs` |
| `crates/argo-cli/src/handlers/workspace_marketplace.rs:313` | `ARGO_OCI_TOKEN` | (new) `tools.marketplace.oci_token` → `NEW: sections_runtime_tools.rs` |

### `argo-cli` — handlers/gateway

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper.rs:33` | `ARGO_APPROVAL_EXPIRY_TICK_SECS` | (new) `lifecycle.approval_expiry_tick_secs` → `NEW: sections_lifecycle.rs` |
| `crates/argo-cli/src/handlers/gateway/approval_reminder.rs:138` | `ARGO_NOTIFY_DEFAULT_EMAIL` | (new) `channels.notify.default_email` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/gateway/boot_spawns.rs:76` | `ARGO_CU_SCREENSHOT_RETENTION_DAYS` | (new) `lifecycle.cu_screenshot_retention_days` → `NEW: sections_lifecycle.rs` |
| `crates/argo-cli/src/handlers/gateway/boot_spawns.rs:125` | `ARGO_DUTY_SCHEDULER_SECS` | `gateway.duty_scheduler_secs` (bound) |
| `crates/argo-cli/src/handlers/gateway/jwt.rs:126` | `ARGO_INBOX_WS_JWT_PUBKEY` | (new) `gateway.jwt.inbox_ws_pubkey_b64` → `NEW: sections_gateway_security.rs` |
| `crates/argo-cli/src/handlers/gateway/jwt.rs:135` | `ARGO_INBOX_WS_JWT_PUBKEY_PEM` | (new) `gateway.jwt.inbox_ws_pubkey_pem` → `NEW: sections_gateway_security.rs` |
| `crates/argo-cli/src/handlers/gateway/rate_limit.rs:290` | `ARGO_DEFAULT_TENANT` | `tenant.default_id` (bound) |
| `crates/argo-cli/src/handlers/gateway/rate_limit.rs:294` | `ARGO_WEBINBOX_TENANT_ID` | (new) `channels.webinbox.tenant_id` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs:124` | `ARGO_IP_RATE_LIMIT` | (new) `gateway.rate_limit.ip.enabled` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs:127` | `ARGO_IP_RATE_CAPACITY` | (new) `gateway.rate_limit.ip.capacity` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs:132` | `ARGO_IP_RATE_REFILL` | (new) `gateway.rate_limit.ip.refill_per_sec` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs:137` | `ARGO_TRUST_XFF` | (new) `gateway.rate_limit.ip.trust_xff` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs:186` | `ARGO_RATE_LIMIT_TENANT_PER_MIN` | `gateway.rate_limit.tenant.requests_per_minute` (field exists; loader needs env binding) |
| `crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs:319` | `ARGO_RATE_LIMIT_TENANT_PER_MIN` | `gateway.rate_limit.tenant.requests_per_minute` (field exists; loader needs env binding) |
| `crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs:377` | `ARGO_GATEWAY_AUTH_BYPASS` | (new) `gateway.auth_bypass` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/routes_auth.rs:30` | `ARGO_GATEWAY_AUTH_BYPASS` | (new) `gateway.auth_bypass` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs:149` | `ARGO_WEBINBOX_OPERATOR_ID` | (new) `channels.webinbox.operator_id` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/gateway/routes_channels_whatsapp.rs:117` | `ARGO_TWILIO_WHATSAPP_DECISION_URL` | (new) `channels.whatsapp.decision_callback_base_url` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/gateway/routes_health.rs:187` | `ARGO_GATEWAY_AUTH_BYPASS` | (new) `gateway.auth_bypass` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs:247` | `ARGO_WEBINBOX_TENANT_ID` / `ARGO_DEFAULT_TENANT` | (new) `channels.webinbox.tenant_id` + bound `tenant.default_id` (single expression chains both) |
| `crates/argo-cli/src/handlers/gateway/routes_peer.rs:67` | `ARGO_GATEWAY_AUTH_BYPASS` | (new) `gateway.auth_bypass` → extend `sections_gateway.rs` |
| `crates/argo-cli/src/handlers/gateway/routes_voice_auth.rs:39` | `ARGO_TWILIO_VOICE_DECISION_URL` | (new) `channels.voice.decision_callback_base_url` → extend `sections_channels.rs` |
| `crates/argo-cli/src/handlers/gateway/security_headers.rs:75` | `ARGO_CSP_REPORT_ONLY` | (new) `gateway.security_headers.csp_report_only` → `NEW: sections_gateway_security.rs` |
| `crates/argo-cli/src/handlers/gateway/shutdown.rs:89` | `ARGO_SHUTDOWN_DEADLINE_SECS` | (new) `lifecycle.shutdown_deadline_secs` → `NEW: sections_lifecycle.rs` |
| `crates/argo-cli/src/handlers/gateway/shutdown.rs:250` | `ARGO_SHUTDOWN_DEADLINE_SECS` | (new) `lifecycle.shutdown_deadline_secs` → `NEW: sections_lifecycle.rs` |
| `crates/argo-cli/src/handlers/gateway/sweepers.rs:32` | `ARGO_HANDOFF_SWEEP_SECS` | `gateway.handoff_sweep_secs` (bound) |

### `argo-executor-document`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-executor-document/src/msgraph.rs:81` | `ARGO_MSGRAPH_ACCESS_TOKEN` | (new) `tools.executor.document.msgraph_access_token` → `NEW: sections_runtime_tools.rs` |

### `argo-genesis`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-genesis/src/refine.rs:100` | `ARGO_GENESIS_REFINE_MAX_ACTIONS` | (new) `genesis.refine.max_actions` → `NEW: sections_genesis.rs` |
| `crates/argo-genesis/src/refine.rs:108` | `ARGO_GENESIS_REFINE_MAX_VOCAB` | (new) `genesis.refine.max_vocab` → `NEW: sections_genesis.rs` |

### `argo-identity`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-identity/src/circuit.rs:51` | `ARGO_CIRCUIT_THRESHOLD` | (new) `identity.circuit_threshold` → `NEW: sections_identity.rs` |

### `argo-mcp-image-gen`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-mcp-image-gen/src/main.rs:40` | `ARGO_HOME` | `core.home` (bound) |

### `argo-security`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-security/src/pii_detect.rs:235` | `ARGO_PII_DETECT` | (new) `security.pii.detect` → extend `sections_security.rs` |
| `crates/argo-security/src/pii_detect.rs:236` | `ARGO_PII_AUTOREDACT` | (new) `security.pii.autoredact` → extend `sections_security.rs` |
| `crates/argo-security/src/pii_detect.rs:237` | `ARGO_PII_LOG_FINDINGS` | (new) `security.pii.log_findings` → extend `sections_security.rs` |

### `argo-store`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-store/src/anomaly.rs:42` | `ARGO_ANOMALY_Z_THRESHOLD` | (new) `store.anomaly_z_threshold` → `NEW: sections_store.rs` |
| `crates/argo-store/src/batched_writer.rs:34` | `ARGO_STORE_BATCH_SIZE` | (new) `store.batch_size` → `NEW: sections_store.rs` |
| `crates/argo-store/src/batched_writer.rs:45` | `ARGO_STORE_BATCH_LATENCY_MS` | (new) `store.batch_latency_ms` → `NEW: sections_store.rs` |
| `crates/argo-store/src/batched_writer.rs:57` | `ARGO_STORE_BATCHED_WRITES` | (new) `store.batched_writes_enabled` → `NEW: sections_store.rs` |
| `crates/argo-store/src/spend_anomaly.rs:67` | `ARGO_SPEND_ANOMALY_Z` | (new) `store.spend_anomaly_z` → `NEW: sections_store.rs` |
| `crates/argo-store/src/worker_archive.rs:304` | `ARGO_WORKER_DELETE_GRACE_DAYS` | (new) `store.worker_delete_grace_days` → `NEW: sections_store.rs` |

### `argo-test-cassette`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-test-cassette/src/recorder.rs:24` | `ARGO_TEST_CASSETTE_MODE` | **out-of-scope for `ArgoConfig`**: cassette mode is set per-test-process and is read by both the recorder and test harnesses; keep as a direct env read and add `crates/argo-test-cassette/` to the exclusion glob in the migration brief (alongside the existing `tests/` / `*_tests.rs` exemptions). Direct read is the contract. |

### `argo-tool`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-tool/src/tools/web/mod.rs:293` | `ARGO_LIVE_WEB` | (new) `tools.web.live` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-tool/src/tools/web/mod.rs:331` | `ARGO_LIVE_WEB` | (new) `tools.web.live` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-tool/src/tools/web/search.rs:59` | `ARGO_DDG_HTML_URL` | (new) `tools.web.ddg_html_url` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-tool/src/tools/web/search.rs:82` | `ARGO_TAVILY_URL` | (new) `tools.web.tavily_url` → `NEW: sections_runtime_tools.rs` |
| `crates/argo-tool/src/tools/web/search.rs:134` | `ARGO_BRAVE_URL` | (new) `tools.web.brave_url` → `NEW: sections_runtime_tools.rs` |

### `argo-tui`

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `crates/argo-tui/src/overlays_h_stack.rs:58` | `ARGO_GATEWAY_URL` | `gateway.url` (bound) |
| `crates/argo-tui/src/overlays_h_stack.rs:157` | `ARGO_GATEWAY_URL` | `gateway.url` (bound; the test-fixture stash of the prior value migrates via test-harness env scaffolding, not a separate `ArgoConfig` field) |

### MCP servers

| Path:line | Env var | Recommended `ArgoConfig` field |
| --- | --- | --- |
| `mcp-servers/voice/src/whisper_native.rs:31` | `ARGO_WHISPER_MODEL` | Keep as MCP env read (out-of-process binary; MCP servers receive config via env, not the host's `ArgoConfig`). The CLI-side `handlers/voice.rs:184` callsite is the migration target. |

## Recommended cluster sections

The migration brief should add the following `sections_<cluster>.rs`
sibling files alongside the existing `sections_budget.rs`,
`sections_channels.rs`, etc. Each clusters cohesive knobs that share
a domain so the migration adds ~5-7 new files rather than ~47
one-off fields scattered across existing sections.

### 1. `NEW: sections_lifecycle.rs` — runtime lifecycle + sweepers

Sweep intervals, shutdown deadline, retention windows. These are
all "how long does X live / how often does X tick" knobs that don't
fit any existing section.

- `lifecycle.shutdown_deadline_secs` — `ARGO_SHUTDOWN_DEADLINE_SECS`
- `lifecycle.approval_expiry_tick_secs` — `ARGO_APPROVAL_EXPIRY_TICK_SECS`
- `lifecycle.cu_screenshot_retention_days` — `ARGO_CU_SCREENSHOT_RETENTION_DAYS`

### 2. `NEW: sections_runtime_tools.rs` — code-exec sandbox, web search, MCP runtime, marketplace, document executor

Knobs that govern *how tools execute at runtime* (sandbox limits,
endpoint URLs, registry-backed mode, MCP health-checking, marketplace
auth token, msgraph document tokens). Distinct from prompt/judge
routing (`sections_routing.rs`) and from the per-channel adapter
config (`sections_channels.rs`).

- `tools.executor.code.timeout_s` — `ARGO_CODE_EXEC_TIMEOUT_S`
- `tools.executor.code.memory_mib` — `ARGO_CODE_EXEC_MEM_MIB`
- `tools.executor.document.msgraph_access_token` — `ARGO_MSGRAPH_ACCESS_TOKEN`
- `tools.web.live` — `ARGO_LIVE_WEB`
- `tools.web.ddg_html_url` — `ARGO_DDG_HTML_URL`
- `tools.web.tavily_url` — `ARGO_TAVILY_URL`
- `tools.web.brave_url` — `ARGO_BRAVE_URL`
- `tools.mcp.health_check_enabled` — `ARGO_MCP_HEALTH_CHECK`
- `tools.mcp.registry_backed` — `ARGO_MCP_REGISTRY_BACKED`
- `tools.marketplace.oci_token` — `ARGO_OCI_TOKEN`

### 3. `NEW: sections_store.rs` — store-layer knobs

Batch writer + anomaly thresholds + retention grace. Concentrated in
the `argo-store` crate and naturally cohesive.

- `store.batch_size` — `ARGO_STORE_BATCH_SIZE`
- `store.batch_latency_ms` — `ARGO_STORE_BATCH_LATENCY_MS`
- `store.batched_writes_enabled` — `ARGO_STORE_BATCHED_WRITES`
- `store.anomaly_z_threshold` — `ARGO_ANOMALY_Z_THRESHOLD`
- `store.spend_anomaly_z` — `ARGO_SPEND_ANOMALY_Z`
- `store.worker_delete_grace_days` — `ARGO_WORKER_DELETE_GRACE_DAYS`

### 4. `NEW: sections_gateway_security.rs` — JWT pubkeys + CSP report-only

JWT pubkey loading (raw and PEM) plus the CSP `report-only` flag.
These could extend `sections_security.rs` or `sections_gateway.rs`;
pulling them into their own sibling keeps each existing section
under 250 LOC and co-locates the gateway-WS-auth knobs.

- `gateway.jwt.inbox_ws_pubkey_b64` — `ARGO_INBOX_WS_JWT_PUBKEY`
- `gateway.jwt.inbox_ws_pubkey_pem` — `ARGO_INBOX_WS_JWT_PUBKEY_PEM`
- `gateway.security_headers.csp_report_only` — `ARGO_CSP_REPORT_ONLY`

### 5. `NEW: sections_genesis.rs` — genesis refine bounds

Two refine-loop bounds for the `argo-genesis` crate. Small but
isolated; extending an unrelated section would be worse than a tight
~30-LOC sibling.

- `genesis.refine.max_actions` — `ARGO_GENESIS_REFINE_MAX_ACTIONS`
- `genesis.refine.max_vocab` — `ARGO_GENESIS_REFINE_MAX_VOCAB`

### 6. `NEW: sections_identity.rs` — identity circuit breaker

Single key today; multiple credential / circuit knobs will likely
accumulate in `argo-identity` post-MVP. Lean toward a sibling now
rather than co-locating with `sections_security.rs` (different
crate, different concern).

- `identity.circuit_threshold` — `ARGO_CIRCUIT_THRESHOLD`

### Extensions to existing sections (no new file needed)

- **`sections_budget.rs`** — `budget.alert_operator_id`,
  `budget.watch_interval_secs`.
- **`sections_channels.rs`** — `channels.hmac_secret`,
  `channels.delivery_reports_path`, `channels.notify.default_email`,
  `channels.webinbox.operator_id`, `channels.webinbox.tenant_id`,
  `channels.webinbox.path`, `channels.voice.whisper_model`,
  `channels.voice.decision_callback_base_url`,
  `channels.whatsapp.decision_callback_base_url`.
- **`sections_core.rs`** — `core.operator`.
- **`sections_gateway.rs`** — `gateway.auth_bypass`,
  `gateway.rate_limit.ip.{enabled,capacity,refill_per_sec,trust_xff}`,
  *plus* a new env binding for the existing
  `gateway.rate_limit.tenant.requests_per_minute` field (the struct
  field exists; the loader does not yet read
  `ARGO_RATE_LIMIT_TENANT_PER_MIN`).
- **`sections_security.rs`** — `security.pii.{detect,autoredact,log_findings}`.

### Out of scope for the migration

- `ARGO_TEST_CASSETTE_MODE` — recorded by `argo-test-cassette` and
  cooperatively read by both the recorder and the test harnesses
  that run the cassettes. Keep as a direct env read; add
  `crates/argo-test-cassette/` to the script's exclusion glob (next
  to the existing `tests/` / `*_tests.rs` rules).
- `mcp-servers/voice/src/whisper_native.rs:31` — MCP servers are
  out-of-process binaries that receive config via env vars, not the
  host's `ArgoConfig`. The CLI-side `handlers/voice.rs:184` callsite
  is the migration target; the MCP server retains its env read.

## Ad-hoc TOML readers (advisory)

The original brief flagged `.skycot/*.toml` and `argo.toml` ad-hoc
loaders as the second wedge of scattered config. The scaffold does
not migrate these; they're listed so the follow-up brief can decide
whether to absorb them into `argo_config::load()` or leave them as
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
and `crates/argo-config/src/loader.rs`) currently binds these env
vars in `from_env()` upfront, so the bound callsites in the table
above migrate as one-liners once the follow-up runs:

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

**~46 of the 52 distinct env-var keys** in this audit need a new
`ArgoConfig` field (or — for the one case of
`ARGO_RATE_LIMIT_TENANT_PER_MIN` — a new env-binding in the loader
where the struct field already exists). Two more keys
(`ARGO_TEST_CASSETTE_MODE`, the MCP-server-side `ARGO_WHISPER_MODEL`)
stay as direct env reads and are documented as exclusions. That
leaves roughly **46 new field bindings + 1 new loader hook + 2
documented exclusions = ~49 migration actions** as the scope estimate
for the follow-up `claude-config-loader-migrate-callsites-v2` brief.

The "~30 of 39" estimate from the original
`claude-config-loader-migrate-callsites` brief understated the
long-tail size by ~50%; sub-brief 2 should be sized accordingly —
likely split further per cluster (one PR per `NEW: sections_*.rs`
sibling above) or run as a manager-worker brief with one child per
cluster.
