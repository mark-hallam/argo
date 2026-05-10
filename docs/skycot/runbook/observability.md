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

---

Next: [Security](security.md) · [Where to next + reference](reference.md).
