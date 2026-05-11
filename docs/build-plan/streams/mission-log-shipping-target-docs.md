# Mission load-bearing wire — Log shipping target runbook

**Owner**: claude. **Depends on**: existing tracing-subscriber wiring (verify via `rg -l 'init_subscriber\|tracing_subscriber::registry' crates/argo-telemetry/src/`); the planned `mission-structured-logging-tenant-worker-session` brief (already queued, defines the tenant_id+worker_id+session_id three-tuple convention).
**Authored against**: origin/main `47d39166` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-log-shipping-target-docs`.

> SOURCE: MISSION_BACKLOG.md Section 1 #34 ("Add log shipping target documentation (Loki, Datadog, CloudWatch)"). **Tier B (operability).** SKYCOT emits structured JSON-line logs through `argo-telemetry`. Operators get the events but no documented recipe for shipping to their log-aggregation stack. Without that, SREs are stuck `tail`-ing the local file. This brief lands the runbook + ONE thin enable-shipping seam in the subscriber.

## Scope

**Doc-heavy brief.** Two new pieces:

1. **NEW runbook** `docs/skycot/runbook/log-shipping.md` (≤ 360 LOC) covering 5 popular targets with working config snippets:
   - **Loki + Promtail** (sidecar tailing `~/.argo/logs/argo.log`, parsing JSON, labeling by `tenant_id` + `worker_id`)
   - **Datadog** (datadog-agent w/ `logs_config.processing_rules` for the SKYCOT-specific `metric = "..."` field; OR direct HTTP via the just-merged http-target seam below)
   - **AWS CloudWatch Logs** (CloudWatch Agent w/ JSON parsing, log-group-per-tenant pattern, retention-policy advice)
   - **Splunk HEC** (HTTP Event Collector, source-type guidance, index recommendations)
   - **Vector.dev** (universal-pipeline option for operators who don't want to commit to one vendor)

   Each section: 1-paragraph context → working YAML/TOML config snippet → 3-5 troubleshooting tips → "verify it's working" command. Mirror the style of existing `docs/skycot/runbook/gateway-tls-deployment.md` (when authored — sibling pattern from the parallel `mission-gateway-tls-termination-or-proxy-doc` brief).

2. **NEW thin shipping seam** `crates/argo-telemetry/src/http_target.rs` (≤ 220 LOC) — an OPTIONAL HTTP-target writer that bypasses the file logging when configured. Operators who can't run a sidecar agent (heroku-style hosting, FaaS) get direct HTTP push:

```rust
pub struct HttpLogTarget {
    endpoint: Url,
    headers: HeaderMap,
    batch_size: usize,           // default 100 events
    flush_interval: Duration,    // default 5s
    timeout: Duration,           // default 10s
    queue: Mutex<VecDeque<String>>, // bounded; drop-oldest on overflow
}

pub struct HttpLogTargetLayer { /* tracing-subscriber Layer wrapping HttpLogTarget */ }

impl Drop for HttpLogTarget { /* best-effort final flush w/ short timeout */ }
```

`[telemetry.log_shipping]` config block:
```toml
[telemetry.log_shipping]
enabled = false
mode = "http_target"           # only mode in v1; sidecar=file is the default
endpoint = "https://logs-vendor.example/v1/ingest"
header_authorization_env = "ARGO_LOGSHIP_AUTH" # env-indirect for secrets
batch_size = 100
flush_interval_seconds = 5
timeout_seconds = 10
queue_max = 10000              # bound; drop-oldest on overflow
```

The HttpLogTarget is INTENTIONALLY simple: best-effort, drop-on-overflow, no on-disk spool. Operators with high-volume / no-loss requirements use the sidecar recipes in the runbook (which DO have on-disk spool via promtail/datadog-agent/cloudwatch-agent).

This is NOT in scope: gzip/zstd batch compression (separate brief once we see traffic justify it); per-tenant log-volume budgets (separate brief); log-line PII redaction (separate brief, more controversial — needs design); on-disk spool for the HttpLogTarget (separate); Otel-logs vs Otel-traces — this is logs-only.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `docs/skycot/runbook/log-shipping.md` (NEW, ≤ 360 LOC)
- `crates/argo-telemetry/src/http_target.rs` (NEW, ≤ 220 LOC)
- `crates/argo-telemetry/src/http_target_tests.rs` (NEW, sibling, ≤ 220 LOC) — 7 unit tests:
  - single-event-flushes-at-flush-interval
  - batch-flushes-at-batch-size-cap-before-interval
  - queue-overflow-drops-oldest-not-newest
  - http-500-doesnt-crash-layer-logs-warn
  - http-timeout-doesnt-block-tracing-call
  - drop-flushes-best-effort-w/-short-timeout
  - tenant-id-field-passes-through-unchanged
- `crates/argo-telemetry/src/lib.rs` — narrowly scoped: register `pub mod http_target;` + install Layer when `[telemetry.log_shipping].enabled = true`
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[telemetry.log_shipping]` section
- `crates/argo-config/src/defaults.rs` — narrowly scoped
- `crates/argo-telemetry/tests/http_target_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke: start a tiny wiremock server, emit 5 tracing events, assert wiremock saw 1 POST with 5 events in the body

## Files you must NOT touch

- `crates/argo-store/migrations/**` — no schema
- `crates/argo-audit/**` — audit chain is separate (different consistency requirements)
- Vertical / frontend / other channels

## Acceptance criteria

- [ ] Runbook covers all 5 targets with working config snippets that an operator could paste and run
- [ ] Each section includes: context, snippet, troubleshooting tips, verify command
- [ ] `HttpLogTarget` flushes at batch_size cap or flush_interval, whichever comes first
- [ ] Queue overflow drops OLDEST events (not newest — new events are more relevant to "what's happening now")
- [ ] HTTP errors (5xx, timeout) log `tracing::warn!` but do NOT crash the layer or block the calling tracing-event
- [ ] `Drop` flushes best-effort with short timeout (default 2s); never blocks shutdown indefinitely
- [ ] `[telemetry.log_shipping].enabled = false` → Layer not registered; true zero-overhead
- [ ] 7 unit tests + 1 integration smoke
- [ ] `cargo clippy -p argo-telemetry -p argo-config --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-telemetry -p argo-config` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-log-shipping-target-docs`
- PR title: `telemetry: log shipping HTTP target + runbook for 5 vendors (mission-log-shipping-target-docs)`

## Notes for the implementer

- The runbook is the LOAD-BEARING artifact; the code seam is the secondary deliverable. If time-budget cuts force a trade-off, ship the runbook with stub "see follow-up brief" placeholders for the code seam — operators who run a sidecar agent are unblocked just by the runbook alone.
- Loki/Promtail: most popular open-source target. Recipe should include the `pipeline_stages.json` block to parse SKYCOT's JSON log format and surface `tenant_id`/`worker_id`/`session_id` as Loki labels.
- Datadog: most popular vendor. The agent config snippet should include the `processing_rules` for the SKYCOT `metric = "..."` event-field convention so dashboards group correctly.
- CloudWatch: the AWS default. Group-per-tenant pattern is the SOC-2 / multi-tenant-isolation requirement; document this prominently.
- Splunk HEC: enterprise pattern; HEC token endpoint shape is `<host>:8088/services/collector/event`; document the source-type.
- Vector.dev: vendor-neutral; mention it as the "I want to hedge between vendors" option with a config that fans out to file+datadog simultaneously.
- The `HttpLogTarget` Layer must NEVER block the calling code's tracing-event emit. Use a bounded `tokio::sync::mpsc` channel for queueing; on full, drop-oldest. The fan-out task runs in `tokio::spawn` and the tracing emit just hits `try_send` (zero-block).
- Token rotation: env-indirect for the auth header (`ARGO_LOGSHIP_AUTH`) so operators can rotate without restart; mirror the pattern used by the Teams/Matrix briefs in this same replenishment batch.
- Smoke test: use `wiremock` (already in workspace) to spin up a localhost HTTP target, emit events, assert receipt. Don't try to test against real Loki/Datadog/CloudWatch — that's operator responsibility.
