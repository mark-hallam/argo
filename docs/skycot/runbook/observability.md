# SKYCOT Operator Runbook — Audit and observability

How to query the audit ledger, run the doctor health check, scrape
Prometheus, ship traces over OTLP, watch live audit streams, measure
coverage, and read the common failure-mode table. For the full table
of contents, see [RUNBOOK.md](../RUNBOOK.md).

---

## 9. Audit and observability

### 9.1 `argo audit query`

```bash
# All actions in the last hour
argo audit query --since-minutes 60

# Only auto-approval decisions
argo audit query --kind auto_approval

# One worker's full trail
argo audit query --worker-id <id> --json | jq '.[]'

# Failed actions
argo audit query --kind action_failed --since-days 7
```

Every row has:

- `kind` — what happened (action_emitted, auto_approved,
  channel_queried, executed, completed, failed, handoff,
  skill_proposed, …)
- `at` — UTC timestamp
- `worker_id`, `action_id`, `session_id` — denormalised so
  you don't have to join
- `payload_json` — the full row at that moment

### 9.2 `argo doctor`

The same command you ran in [Bootstrap](getting-started.md#3-bootstrap)
works as an ongoing health check:

```bash
argo doctor --json | jq '.summary'
```

```json
{
  "verticals_loaded": 5,
  "workers_active": 3,
  "actions_pending": 12,
  "actions_blocked": 0,
  "memory_recent_rows": 847,
  "memory_indexed_rows": 1239,
  "memory_summary_rows": 41,
  "audit_rows_today": 312,
  "auto_approval_today": 28,
  "channel_notify_today": 4
}
```

### 9.3 Prometheus

When the gateway is running:

```bash
argo gateway serve --port 7676 &
curl http://localhost:7676/metrics | grep argo_
```

Metrics shipped:

- `argo_actions_emitted_total{vertical,kind}`
- `argo_auto_approval_decisions_total{decision}`
- `argo_handoffs_total{from,to,outcome}`
- `argo_evaluation_latency_seconds` (histogram)
- `argo_workers_active`
- `argo_memory_layer_rows{layer}`

Wire to whatever Prometheus + Grafana you already use. The
metrics names are stable — `argo_*` is the namespace.

### 9.3.1 OpenTelemetry / OTLP exporter

`argo-telemetry` ships an optional OpenTelemetry Protocol (OTLP)
exporter behind the `otlp` cargo feature (default off — single-
binary distribution stays zero-deps without it). When the feature
is on, the in-loop spans (`agent.turn`, `judge.adjudicate`, …) and
counters / histograms (`routing_decisions_total`,
`judge_duration_ms`, …) are routed to whatever OTel collector you
point them at.

Build with the feature on:

```bash
cargo build --release --features argo-telemetry/otlp
```

Then on process start, call `argo_telemetry::otlp_exporter::init()`
once and hold the returned `ShutdownGuard` for the lifetime of the
process — its `Drop` flushes any pending batches synchronously.

Two environment variables tune the exporter:

- `OTEL_EXPORTER_OTLP_ENDPOINT` — collector base URL. Default
  `http://localhost:4318` (OTLP/HTTP standard port). Spans POST to
  `<endpoint>/v1/traces`, metrics POST to `<endpoint>/v1/metrics`.
- `OTEL_SERVICE_NAME` — `service.name` resource attribute.
  Default `argo`.

Point at a collector by setting the endpoint:

```bash
# Local OTel collector / Datadog Agent / any OTLP/HTTP receiver
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 argo gateway serve

# Hosted backend that accepts OTLP/HTTP (e.g. Honeycomb)
OTEL_EXPORTER_OTLP_ENDPOINT=https://api.honeycomb.io \
OTEL_SERVICE_NAME=argo-prod \
argo gateway serve
```

For backends that need auth headers (e.g. Honeycomb's
`X-Honeycomb-Team`), front the collector with a sidecar OTel
collector that adds the headers — first-party header config is a
follow-up item.

#### Bridging `#[instrument]` spans automatically

In addition to the in-loop spans wired through `TracingTelemetry`,
the OTLP exporter can pick up every `#[instrument]`-decorated
function across the workspace via the `tracing-opentelemetry`
bridge layer. After calling `otlp_exporter::init()`, install
`argo_telemetry::otlp_bridge::otlp_layer()` on your
`tracing-subscriber` registry:

```rust,ignore
use tracing_subscriber::prelude::*;

let _guard = argo_telemetry::otlp_exporter::init()?;
tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .with(argo_telemetry::otlp_bridge::otlp_layer())
    .init();
```

Span attributes recorded via `#[instrument(fields(...))]` propagate
as OTLP span attributes. The bridge is feature-gated under `otlp`
so default builds incur no extra dependencies.

### 9.4 Live audit stream

For real-time monitoring (live ops dashboard, on-call):

```bash
# Subscribe to the gateway's audit stream — every audit row
# pushed as a WebSocket frame.
argo gateway tail --kind auto_approval,handoff
```

(Surface still landing as `claude-realtime-audit-stream`; the
existing `argo audit query --watch` flag is the today-state
substitute that polls every second.)

### 9.5 Coverage

CI measures Rust coverage on every PR and main push with
`cargo-llvm-cov`. The job uploads `lcov.info` and
`coverage-summary.txt`, comments the summary on PRs, and uploads
`coverage.svg` as an artifact on main pushes. No threshold is
enforced yet; this is a measurement baseline. The coverage job
ignores test-run failures while collecting coverage so known
pre-existing flakes do not suppress the report; the CI `test`
job remains the correctness gate.

To run the same report locally:

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
cargo llvm-cov --workspace --html
```

Open `target/llvm-cov/html/index.html` for the annotated report.

### 9.6 Common failure modes

A condensed copy of this table — plus the cli command index from
[Reference Appendix A](reference.md#appendix-a--cli-command-index) —
is rendered to a self-contained one-screen card at
[`../quickref.html`](../quickref.html). Regenerate with
`bash scripts/gen-runbook-quickref.sh`.

<!-- quickref:errors:start -->
| Symptom | Cause | Fix |
|---|---|---|
| `executor 'computer_use' is not yet implemented` | `E2B_API_KEY` or `ANTHROPIC_API_KEY` not set | Set them, or declare `executor = "human"` to fall back to the artifact-bundle path |
| `unknown executor kind 'browser'` | Manifest typo — only `human \| mcp \| computer_use \| api` are valid | Fix the action's `executor` field |
| `safety judge rejected the generated manifest` | An action with a sensitive verb is `requires_approval = false` | The auto-fix should have caught this; if you see it, the manifest was hand-edited after generation. Set `requires_approval = true` on the offending action. |
| `vertical-id 'marketing_intern' must be lowercase ASCII letters, digits, or '-'` | Underscores in the id | Use dashes: `marketing-intern-maya`. Genesis converts automatically; only hand-authored manifests hit this. |
| `output dir '...' is not empty` | Re-running `argo genesis create` against an existing dir | Use a fresh path or `rm -rf` the old one — Genesis refuses to clobber. |
| `action {id}: approval required (state pending, requires_approval=true)` | Trying to execute before approval | Run `argo worker actions approve <id>` first, or set up an auto-approval policy |
<!-- quickref:errors:end -->

### 9.7 Startup health check (exit 78)

`argo gateway start` runs a synchronous pre-bind health check before
the TCP listener accepts. It verifies (1) the SQLite store is
reachable, (2) `schema_version` matches the binary's expected
version, (3) at least one of `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`
is set, and (4) lists configured channel adapters (warn-only).

On critical failure the gateway prints `STARTUP CHECK FAILED: <reason>`
to stderr and exits **78** (BSD `EX_CONFIG`). Pod orchestrators
treat 78 as a configuration problem rather than crash-loop spam.

| Stderr message fragment | Meaning | Fix |
|---|---|---|
| `store unreachable` | SQLite open / `SELECT 1` failed | Check that `ARGO_HOME` exists and is writable |
| `schema is behind the binary` | DB at older `schema_version` | Run `argo db migrate` |
| `schema is ahead of the binary` | DB at newer `schema_version` | Roll the binary back to the matching release |
| `no provider API key set` | Neither Anthropic nor OpenAI key in env | Set `ANTHROPIC_API_KEY` or `OPENAI_API_KEY` |

For dev / recovery only, `--skip-startup-check` bypasses every
check. Don't ship that flag to production.

### 9.8 Log aggregation

`argo gateway serve` writes log lines to **stderr** by default, formatted
by [`SecretsScrubberLayer`](../../crates/argo-cli/src/secrets_scrubber.rs).
This section covers shipping those logs to a real aggregator (Loki,
Grafana, ELK), file rotation, and the field reference operators need to
write parsers and dashboards.

**Trace data vs log data — keep them separate.** Spans, counters, and
histograms ship via the OTLP exporter (see [§9.3.1](#931-opentelemetry--otlp-exporter)
and [#938 mission-otlp-exporter](https://github.com/anthropics/argo/pull/938)).
This section is only about the textual log stream — `tracing::info!` /
`error!` / etc. lines emitted to stderr. The two paths are intentionally
split: traces → OTLP collector, logs → stdout/file → Promtail → Loki.
Don't try to scrape spans out of stderr or push log lines through OTLP.

#### 9.8.1 Output format and env vars

Two env vars control current output:

- `RUST_LOG` — standard `tracing-subscriber` env-filter directive.
  Default `warn,argo=info` (warn floor, info for `argo` targets).
  Examples: `RUST_LOG=info`, `RUST_LOG=argo_runtime=debug,warn`.
- `ARGO_TRACING_SCRUB` — set to `0` to disable the secrets-scrubbing
  layer. **Production must leave this unset.** When `0`, output falls
  back to `tracing-subscriber`'s plain `fmt::layer()` and the binary
  prints a `secrets scrubbing DISABLED` warning to stderr.

Today's line format (scrubber on, the production path) is:

```
<LEVEL> <target> <message> <key>=<value> <key>=<value> ...
```

Example:

```
INFO argo_gateway::serve listening port=7676
WARN argo_runtime::router provider fell back from anthropic to openai reason=rate_limit attempt=2
```

> **Status — structured JSON output:** not yet wired (verified
> 2026-05-11 with `rg -n 'tracing.*json' crates/argo-cli/src` — no
> matches in `tracing_setup.rs`; `SecretsScrubberLayer` formats
> events as plain text). The brief's intent was to document an
> existing JSON flag; that flag does not exist on main. JSON-mode
> wiring (timestamp, span ids, trace ids in a stable envelope) is a
> follow-up code brief — track as `claude-tracing-json-mode`. The
> Field reference below documents both today's text-format keys and
> the planned JSON envelope so operators can choose a parser strategy
> now and migrate forward without re-writing label maps.

#### 9.8.2 File rotation

The simplest production path is to redirect stderr to a file under
systemd's logging or a logrotate-managed path. Both are stable
operating-system primitives — they do not depend on any argo build
flag.

**Option A — `logrotate` (Linux, BSD).** Add the snippet to
`/etc/logrotate.d/argo`:

```
/var/log/argo/gateway.log {
    daily
    rotate 14
    missingok
    notifempty
    compress
    delaycompress
    copytruncate
    su argo argo
}
```

`copytruncate` is required because `argo gateway serve` does not
re-open its log fd on `SIGHUP` today. Daily + 14-day retention is the
SKYCOT default; tune `rotate` and `daily`/`hourly` to your storage
budget.

**Option B — `journald` (systemd).** If you run argo as a systemd
unit, journald rotates and indexes for free. Example unit drop-in:

```ini
[Service]
StandardOutput=journal
StandardError=journal
SyslogIdentifier=argo-gateway
```

Tail with `journalctl -u argo-gateway -f`. Promtail can pull from
journald directly via its `journal:` source; see the example file
in `docs/skycot/log-aggregation/promtail-example.yaml`.

#### 9.8.3 Shipping to Loki + Grafana

Promtail is the recommended shipper for Loki because it handles
rotation, label management, and back-pressure cleanly. A complete
ready-to-paste config lives at
[`docs/skycot/log-aggregation/promtail-example.yaml`](../log-aggregation/promtail-example.yaml).
Two label conventions to keep in mind:

- **Static labels only:** `app="argo"`, `component="gateway"`,
  `env="${ENVIRONMENT}"`. These three identify the stream.
- **No per-session, per-worker, or per-action labels.** Loki's index
  cardinality is the dominant cost — keep dynamic data in the line
  body, not the labels. Use `logfmt` parsing in your LogQL queries
  to filter by `worker_id` / `action_id` at query time.

For Grafana, point a Loki datasource at your collector and start with:

```
{app="argo", component="gateway"} |= "error"
{app="argo"} | logfmt | level="WARN"
```

#### 9.8.4 Field reference

Two-column reference: which keys are stable in **today's text format**
(scrubber output) versus the **planned JSON envelope** (follow-up).
Operators writing a parser today should target the text-format
column; the JSON-mode rollout will preserve the same key names so
dashboards and alerts migrate cleanly.

| Key | Today (text) | Planned (JSON) | Notes |
|---|---|---|---|
| `timestamp` | not in line (use ingest time) | RFC 3339 UTC | Ingest-time timestamp from journald / Promtail is the substitute today |
| `level` | first whitespace token | string `"INFO"` etc. | Stable; match against `INFO\|WARN\|ERROR\|DEBUG\|TRACE` |
| `target` | second whitespace token | string | Tracing target — module path, e.g. `argo_runtime::router` |
| `msg` | text between target and first `k=v` | string | Free-form message; may contain spaces |
| `span_name` | not present | string | Innermost span at event time (planned) |
| `span_id` | not present | hex string | OTLP-compatible span id (planned) |
| `trace_id` | not present | hex string | OTLP-compatible trace id (planned) |
| `fields` | space-separated `k=v` tail | object | Per-event `tracing` fields; values may contain `<scrubbed:*>` markers |

Three caveats:

1. **Scrubbed values.** Any field whose name matches `password`,
   `api_key`, `bearer`, `*_secret`, `access_token`, `refresh_token`
   (and a few inline patterns — JWTs, `Bearer …`) is replaced with
   `<scrubbed:kind>` before any writer sees it. Don't write alerts
   that match on the scrubbed marker — match on the field key
   instead.
2. **Field names are best-effort.** `tracing::info!(worker_id = "…")`
   call sites use whatever key the author wrote. The runtime does
   not enforce a schema; treat unfamiliar keys as informational, not
   missing.
3. **Multi-line messages.** Errors with embedded newlines (panic
   backtraces in particular) span multiple log lines. Promtail's
   default config in `promtail-example.yaml` includes a multiline
   stage that joins continuation lines until the next leading
   `INFO|WARN|...` token.

---
