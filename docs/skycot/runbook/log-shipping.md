# SKYCOT Operator Runbook — Log shipping

Ship SKYCOT's structured JSON-line logs (`~/.argo/logs/argo.log`) to
your aggregation stack. Each line carries `tenant_id` / `worker_id` /
`session_id`, an `at` ISO-8601 timestamp, a `level`, a SKYCOT-specific
`metric` field (`counter`, `histogram`, plain), and the call-site
event fields. The recipes below surface those as labels / tags /
indexed fields per vendor.

The audit trail is **not** shipped by this pipeline (different
consistency requirements; SOC-2 separation-of-duty). Audit export is
its own command (`argo audit export`).

## Choosing a target

| Operator profile                       | Recommended target                  |
|----------------------------------------|-------------------------------------|
| Self-hosted; cost-sensitive            | **Loki + Promtail** sidecar         |
| Datadog SaaS customer                  | **Datadog Agent** sidecar           |
| AWS-native deployment                  | **CloudWatch Agent** sidecar        |
| Enterprise / regulated                 | **Splunk HEC** push                 |
| Multi-target / vendor-hedge            | **Vector.dev** pipeline             |
| Heroku / FaaS (no sidecar)             | Direct HTTP (`http_target` Layer)   |

Sidecar paths read SKYCOT's existing on-disk log file unchanged. The
direct-HTTP path bypasses the file and POSTs JSON arrays straight to
the endpoint — best-effort, no on-disk spool, drop-oldest on overflow.

---

## 1. Loki + Promtail (open source, cost-efficient)

Most popular open-source stack. Promtail tails the SKYCOT log file,
parses each JSON line, and forwards to Loki with `tenant_id`,
`worker_id`, and `session_id` exposed as Loki labels.

**`/etc/promtail/config.yml`:**

```yaml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /var/lib/promtail/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: skycot
    static_configs:
      - targets: [localhost]
        labels:
          job: skycot
          __path__: /home/skycot/.argo/logs/argo.log
    pipeline_stages:
      - json:
          expressions:
            tenant_id: tenant_id
            worker_id: worker_id
            session_id: session_id
            level: level
            metric: metric
            at: at
      - labels:
          tenant_id:
          worker_id:
          level:
      - timestamp:
          source: at
          format: RFC3339Nano
```

**Troubleshooting:**

- `level=info` lines but no `tenant_id` label → multi-tenant mode is
  off; `argo.toml` `[tenant].mode = "single"` puts all events under
  `tenant_id = "default"`.
- Promtail rotating the log mid-write → set
  `log_rotation_compatible: true` in the `scrape_configs` block.
- "label cardinality high" warning from Loki → drop `session_id`
  from the `labels` list; keep it as a JSON field, query with
  `| json | session_id="..."`.
- Promtail behind SKYCOT after restart → check
  `positions.yaml`; delete to force a full re-tail.

**Verify:**

```bash
logcli --addr http://loki:3100 query '{job="skycot",tenant_id="acme"}' --limit 5
```

---

## 2. Datadog (most popular SaaS vendor)

The Datadog Agent tails the SKYCOT log file and parses each JSON line.
Processing rules surface the SKYCOT `metric` field so dashboards group
correctly. Two modes — pick one:

- **A. Sidecar agent** (recommended): handles spool / retries /
  compression.
- **B. Direct HTTP via `[telemetry.log_shipping]`**: lower setup cost,
  no on-disk spool, for hosts that cannot run a sidecar.

**A. Sidecar `/etc/datadog-agent/conf.d/skycot.d/conf.yaml`:**

```yaml
logs:
  - type: file
    path: /home/skycot/.argo/logs/argo.log
    service: skycot
    source: skycot
    log_processing_rules:
      - type: mask_sequences
        name: mask_bearer_tokens
        pattern: 'Bearer [A-Za-z0-9\-\._~\+\/=]+'
        replace_placeholder: 'Bearer [REDACTED]'
      - type: multi_line
        name: not_multiline
        pattern: '^\{'
```

Datadog auto-parses JSON when `source: skycot` is set and the line
starts with `{`. Tags get a `skycot:` prefix.

**B. Direct HTTP (`argo.toml`):**

```toml
[telemetry.log_shipping]
enabled = true
mode = "http_target"
endpoint = "https://http-intake.logs.datadoghq.com/api/v2/logs"
header_authorization_env = "ARGO_LOGSHIP_AUTH"  # set to "DD-API-KEY xxxx"
batch_size = 100
flush_interval_seconds = 5
timeout_seconds = 10
queue_max = 10000
```

Set `ARGO_LOGSHIP_AUTH="DD-API-KEY ${DD_API_KEY}"` in the environment.

**Troubleshooting:**

- Logs in Datadog show `status=info` but no `tenant_id` tag → set
  `service: skycot` and use a search facet on `@tenant_id`.
- Sidecar agent at 95% disk-buffer → raise
  `logs_config.open_files_limit` from 200; check disk space.
- Direct-HTTP path showing 401 → `DD-API-KEY` value wrong; the env
  var carries the full header value, not just the key.

**Verify:**

```bash
# Datadog UI: Logs Explorer → service:skycot
curl -s "https://api.datadoghq.com/api/v2/logs/events/search?filter%5Bquery%5D=service%3Askycot&filter%5Bfrom%5D=now-15m" \
  -H "DD-API-KEY: $DD_API_KEY" -H "DD-APPLICATION-KEY: $DD_APP_KEY" | jq '.data | length'
```

---

## 3. AWS CloudWatch Logs (AWS-native)

The CloudWatch Agent tails the SKYCOT log file and ships to
CloudWatch Logs. **Use one log-group per tenant** — this is the
SKYCOT multi-tenant isolation requirement and the operational shape
SOC-2 audits expect.

**`/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json`:**

```json
{
  "logs": {
    "logs_collected": {
      "files": {
        "collect_list": [
          {
            "file_path": "/home/skycot/.argo/logs/argo.log",
            "log_group_name": "/skycot/{tenant_id}",
            "log_stream_name": "{worker_id}/{session_id}",
            "retention_in_days": 30,
            "timezone": "UTC",
            "timestamp_format": "%Y-%m-%dT%H:%M:%S.%fZ",
            "timestamp_regex": "\"at\":\"([0-9T:\\.\\-Z]+)\""
          }
        ]
      }
    }
  }
}
```

Placeholders `{tenant_id}` / `{worker_id}` / `{session_id}` are
filled by parsing the JSON line; the agent reads the value out of the
same line that becomes the log payload.

IAM least-privilege: actions `logs:CreateLogGroup`,
`logs:CreateLogStream`, `logs:PutLogEvents`, `logs:DescribeLogStreams`
on resource `arn:aws:logs:*:*:log-group:/skycot/*`.

**Troubleshooting:**

- `ResourceNotFoundException` on `PutLogEvents` → pre-create with
  `aws logs create-log-group --log-group-name /skycot/<tenant>`.
- Retention not applied to existing groups → `retention_in_days`
  applies to new groups only; use `aws logs put-retention-policy`.
- All events under `tenant_id="default"` → multi-tenant mode off in
  `argo.toml`; turn it on or accept the single-group shape.

**Verify:**

```bash
aws logs tail /skycot/acme-co --since 5m --format short
```

---

## 4. Splunk HEC (enterprise pattern)

Splunk HTTP Event Collector. Use the **event** endpoint (not raw)
because SKYCOT emits one structured event per line. Source-type
`_json` makes Splunk auto-index the JSON fields.

**`argo.toml`:**

```toml
[telemetry.log_shipping]
enabled = true
mode = "http_target"
endpoint = "https://splunk.example.com:8088/services/collector/event"
header_authorization_env = "ARGO_LOGSHIP_AUTH"  # set to "Splunk <hec-token>"
batch_size = 100
flush_interval_seconds = 5
timeout_seconds = 10
queue_max = 10000
```

The default HEC token shape is `Splunk <token-uuid>` — env-indirect
via `ARGO_LOGSHIP_AUTH` keeps the token out of `argo.toml`. Rotate by
re-setting the env var and signaling SKYCOT — no config edit needed.

**Splunk-side index recommendations:**

- Index: `skycot_ops` (separate from `main`; do not mix with the
  audit index, which has different retention requirements).
- Source: `skycot:agent`.
- Source-type: `_json`.
- Default field-extraction on: `tenant_id`, `worker_id`,
  `session_id`, `metric`, `level`.

**Troubleshooting:**

- HEC 403 → token wrong or HEC disabled; verify with
  `curl -k https://splunk.example.com:8088/services/collector/health -H "Authorization: $ARGO_LOGSHIP_AUTH"`.
- Events show up but JSON not extracted → source-type is not `_json`
  or the HEC endpoint is `…/event` not `…/raw`.
- Drop-oldest warnings in SKYCOT logs → `queue_max` too low for
  traffic, OR Splunk is rate-limiting; raise `queue_max` first.

**Verify:**

```bash
# Splunk SPL via the search REST API:
curl -sk -u "$SPLUNK_USER:$SPLUNK_PASS" -X POST \
  "https://splunk.example.com:8089/services/search/jobs/export" \
  --data-urlencode "search=search index=skycot_ops | head 5" \
  --data-urlencode "output_mode=json_rows"
```

---

## 5. Vector.dev (vendor-neutral pipeline)

Use Vector when you need to **fan out** to multiple targets, or to
hedge between vendors during a migration. Vector tails the SKYCOT
log file, parses JSON, and routes to N sinks in parallel.

**`/etc/vector/vector.toml`:**

```toml
[sources.skycot]
type = "file"
include = ["/home/skycot/.argo/logs/argo.log"]
read_from = "beginning"

[transforms.parse]
type = "remap"
inputs = ["skycot"]
source = """
  . = parse_json!(.message)
  .timestamp = parse_timestamp!(.at, "%+")
"""

[sinks.datadog]
type = "datadog_logs"
inputs = ["parse"]
default_api_key = "${DD_API_KEY}"
compression = "gzip"

[sinks.file_archive]
type = "file"
inputs = ["parse"]
path = "/var/log/skycot-archive/%Y-%m-%d.log"
encoding.codec = "json"
```

This fan-out keeps a local archive AND ships to Datadog at the same
time — useful during vendor evaluations and the SOC-2 "redundant
copy of operational logs" control.

**Troubleshooting:**

- Vector restarting in a loop → `vector validate /etc/vector/vector.toml`;
  most common cause is a missing sink env var.
- Datadog sink dropping events → buffer is in-memory by default; add
  `buffer.type = "disk"` with `buffer.max_size` for back-pressure.
- High-cardinality label warnings (Loki sink) → drop `session_id`
  from labels; keep as a JSON field.

**Verify:**

```bash
vector tap --inputs-of parse | head -3
```

---

## Direct HTTP (no sidecar): the `http_target` Layer

When you cannot run a sidecar (Heroku-style hosting, FaaS, container
images that exclude package managers), enable the in-process HTTP
shipping Layer. It is **best-effort**: no on-disk spool, drop-oldest
on overflow, HTTP errors warn-not-crash.

Minimum `argo.toml`:

```toml
[telemetry.log_shipping]
enabled = true
mode = "http_target"
endpoint = "https://logs.example.com/v1/ingest"
header_authorization_env = "ARGO_LOGSHIP_AUTH"
```

Defaults: `batch_size = 100`, `flush_interval_seconds = 5`,
`timeout_seconds = 10`, `queue_max = 10000`.

**Operational characteristics:**

- The Layer **never blocks** the calling `tracing` event emit.
- The flusher task wakes on `flush_interval_seconds` ticks OR when
  the queue reaches `batch_size`, whichever comes first.
- Queue full → OLDEST event drops (new events matter more for
  "what's happening now"). Watch the SKYCOT log for `log-shipping
  non-2xx` warnings if you see drops in your aggregator.
- Process shutdown → best-effort drain bounded by a 2s timeout;
  events still queued after that are lost. SIGKILL bypasses this.

For high-volume / no-loss requirements, use one of the sidecar
recipes above. The sidecar agents all run an on-disk spool and
retries that this Layer intentionally does not.
