//! `argo-siem` — periodic SIEM dispatcher (Mission #227).
//!
//! Tails `audit_record` ordered by `seq` and ships rows to one of
//! three HTTP-shaped SIEM ingest endpoints: Splunk HEC
//! ([`splunk::SplunkHecSink`]), Elasticsearch `_bulk`
//! ([`elastic::ElasticSink`]), or Datadog Logs v2
//! ([`datadog::DatadogLogsSink`]). The
//! [`dispatcher::SiemDispatcher`] tokio task batches per sink,
//! retries 5xx with exponential backoff, and persists the
//! high-water `seq` to `siem_dispatch_cursor` in `argo-store` so a
//! crash + restart resumes from `last_seq + 1` without replay or
//! duplicates. Opt-in via `[siem] enabled = true`; default-off
//! ships zero overhead.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod datadog;
pub mod dispatcher;
pub mod elastic;
pub mod sink;
pub mod splunk;

pub use datadog::DatadogLogsSink;
pub use dispatcher::{spawn_siem_dispatcher, SiemDispatcher, SiemDispatcherConfig};
pub use elastic::ElasticSink;
pub use sink::{AuditEvent, SinkError, SiemSink};
pub use splunk::SplunkHecSink;
