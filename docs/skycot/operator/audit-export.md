# `argo audit export` — SIEM ingestion

`argo audit export` dumps the hash-chained audit log to CSV, JSONL,
JSON, NDJSON, or a signed `.tar` bundle for compliance review and
SIEM ingestion. Streaming for the row-per-line / single-array
formats keeps the operator process bounded regardless of result
size (~100k row exports stay flat-memory).

Verification runs before any bytes are emitted — if
`verify_all` reports a break the export fails and stderr names the
first offending `seq`. Email redaction (`--gdpr-redact-emails`) is
applied per-row before formatting; the canonical-form JSON column
matches the bytes that feed the chain hash so downstream tools can
recompute hashes from the export.

## Synopsis

```text
argo audit export \
  --format csv|jsonl|json|ndjson|tarball \
  [--since 7d | 2026-05-01 | 2026-05-01T00:00:00Z] \
  [--until 2026-05-15] \
  [--tenant ws-default] \
  [--worker-id worker-a] \
  [--kind tool_call,tool_result] \
  [--session <uuid>] \
  [--gdpr-redact-emails] \
  [--bundle-out audit-2026-05.csv] \
  [--json]
```

- `--since` accepts compact durations (`30m`, `24h`, `7d`),
  `YYYY-MM-DD`, or RFC 3339. Duration values are relative to the
  command's `Utc::now()`.
- `--kind` takes a comma list of `AuditKind` labels (`tool_call`,
  `tool_result`, `turn_start`, `judge.verdict`, `rbac.deny`, …).
  Run `argo audit by-kind --json` to enumerate labels actually
  present in the chain.
- `--bundle-out` writes to a path; without it the command streams
  to stdout. `--json` emits a one-line summary envelope on stderr
  for orchestration (rows written, byte count, sha256).

## CSV column reference

| Column              | Source                                              |
| ------------------- | --------------------------------------------------- |
| `seq`               | Append-only audit sequence number.                  |
| `id`                | Hex UUIDv7 row id.                                  |
| `session_id`        | Hex 16-byte session UUID, blank for non-session.    |
| `vertical_id`       | LEFT JOIN through `session.vertical_id`.            |
| `kind`              | `AuditKind` label (`tool_call`, etc.).              |
| `created_at_micros` | UTC epoch microseconds.                             |
| `payload_json`      | Canonical-form JSON — keys sorted, no whitespace.   |
| `parent_hash`       | Hex sha256 of previous row in the chain.            |
| `self_hash`         | Hex sha256 of this row.                             |
| `tenant_id`         | Extracted from payload (`/tenant_id`).              |
| `worker_id`         | Extracted from payload (`/worker_id`, etc.).        |
| `timestamp`         | RFC 3339 form of `created_at_micros`.               |

RFC 4180 quoting: fields containing `,`, `"`, CR, or LF are wrapped
in `"`; embedded `"` is doubled. Splunk, Datadog Logs, and Sumo Logic
all accept this layout without preprocessing.

## Splunk ingestion

```bash
argo audit export --format csv --since 24h --tenant ws-default \
  --bundle-out /var/spool/splunk/audit-$(date +%F).csv
```

Splunk's `csv` source type auto-detects the header row. Field
extraction for nested payload data: configure a `props.conf` stanza
with `INDEXED_EXTRACTIONS = csv` on the source and add
`KV_MODE = json` so payloads are queryable as
`spath input=payload_json output=tool_name path=tool`.

## Datadog Logs (Agent tail)

```bash
argo audit export --format jsonl --since 1h \
  --bundle-out /var/log/argo/audit.jsonl
```

Datadog's Agent JSON line parser ingests each row as a structured
event. Map `kind` → `evt.name`, `tenant_id` → `dd.tenant`, and the
payload's `worker_id` → `dd.worker` via a pipeline processor. Use
`--gdpr-redact-emails` if the log retention bucket is outside the
EU residency boundary.

## Sumo Logic HTTP source

```bash
argo audit export --format json --kind tool_call,tool_result \
  --since 7d | \
  curl -s -X POST -H 'Content-Type: application/json' --data @- \
       "$SUMO_HTTP_SOURCE_URL"
```

The single-array `--format json` form is one POST per export window
— suitable for HTTP-source collectors that prefer batched payloads.
Streaming is preserved end-to-end: the curl body is fed by the
export's `[ … ]` stream.

## Operational notes

- The handler fully verifies the chain before emitting any bytes;
  on a break the file at `--bundle-out` is never opened.
- `--json` envelope is printed on stdout *after* the rows are
  written (for stdout exports the envelope follows the records).
  Run with `--bundle-out` plus `--json` for a clean machine-
  readable summary (rows + sha256) over orchestration channels.
- Schedule via cron: rotate the bundle path daily; pair with
  `argo audit archive` for offline retention.
