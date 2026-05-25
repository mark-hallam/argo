# eval-argo-siem-crate-surface

Date: 2026-05-24
Brief: `eval-argo-siem-crate-surface`
Verdict: PASS

## Scope

Audited the public `argo-siem` crate surface that backs the optional
SIEM dispatcher for Splunk HEC, Elasticsearch `_bulk`, and Datadog
Logs. The audit is contract-only: no production code, store
migrations, gateway boot paths, or live SIEM services were changed.

## Pinned Contracts

- Public exports from `crates/argo-siem/src/lib.rs`: sink modules,
  `SiemEvent`, `SiemSink`, `SinkError`, three concrete HTTP sinks,
  `SiemDispatcher`, `SiemDispatcherConfig`, and
  `spawn_siem_dispatcher`.
- Canonical `SiemEvent` serde shape: `seq`, `id`, `session_id`,
  `kind`, `payload`, and `created_at`.
- Sink request-body format without network calls:
  - Splunk: one HEC envelope per NDJSON line, `source = "argo"`,
    default `sourcetype = "argo:audit"`, and event payload nesting.
  - Elasticsearch: alternating `_bulk` action/document NDJSON lines,
    default action shape, event document fields, and `@timestamp`.
  - Datadog: JSON array body with `ddsource`, `service`, stable
    message text, `timestamp_ms`, and event payload fields.
- Secret-bearing sink `Debug` output remains redacted.
- Dispatcher defaults keep the 15-second interval.
- Dispatcher cursor behavior persists the high-water `seq` by
  backend and endpoint, resumes after restart without replaying
  already-shipped rows, and continues with new rows.
- `SinkError` display strings preserve retry-classification words:
  transient, permanent, and config.

## Findings

No new product or security findings were found.

The brief asked for default-off behavior if exposed. Default-off is
owned by `argo-config::SiemConfig` and gateway boot
`spawn_siem_from_config`, not by `argo-siem`'s public API. This audit
therefore pins the in-crate dispatcher default interval and documents
the boundary rather than widening into config/gateway files.

## Verification

- `CARGO_TARGET_DIR=/tmp/eval-argo-siem-crate-surface-target cargo test -p argo-siem --test surface_contract`
- `CARGO_TARGET_DIR=/tmp/eval-argo-siem-crate-surface-target cargo test -p argo-siem --tests`
- `CARGO_TARGET_DIR=/tmp/eval-argo-siem-crate-surface-target cargo clippy -p argo-siem --all-targets -- -D warnings`
