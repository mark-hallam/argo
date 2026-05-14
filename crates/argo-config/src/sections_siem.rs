//! `[siem]` config section — opt-in SIEM dispatcher (Splunk HEC /
//! Elasticsearch `_bulk` / Datadog Logs). Mirrors the sibling-module
//! convention from [`crate::sections`] (one section file per concern,
//! each ≤ 250 LOC).
//!
//! Default `enabled = false` so existing operators see zero overhead
//! from the new dispatcher — no task spawned, no schema touch, no
//! HTTP client. Operators flip the flag + set `backend` / `endpoint`
//! / `token` to ship.
use serde::{Deserialize, Serialize};

/// `[siem]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SiemConfig {
    /// Master enable. Off by default; flipping on spawns the
    /// dispatcher under the gateway's existing sweeper conditional.
    pub enabled: bool,
    /// One of `"splunk"`, `"elastic"`, `"datadog"`. v1 ships one
    /// sink per gateway — operators with multiple SIEMs run two
    /// gateways. Any unrecognised value is treated as a config
    /// error at dispatcher construction.
    pub backend: String,
    /// Ingest endpoint. For Splunk: the cluster base URL — the sink
    /// appends `/services/collector`. For Elastic: the cluster base
    /// URL — the sink appends `/_bulk`. For Datadog: the full v2
    /// logs intake URL (operators in EU regions point at the EU
    /// host).
    pub endpoint: String,
    /// Credential. Splunk HEC token / verbatim Elastic
    /// `Authorization` header value / Datadog API key. Never
    /// rendered in `Debug` or `tracing` field output — the
    /// dispatcher wraps in a `SecretString` before storing.
    pub token: String,
    /// Optional `sourcetype` (Splunk) / index name (Elastic) /
    /// `ddsource` (Datadog) override. Each sink picks the field
    /// relevant to its backend; the others are ignored.
    pub sourcetype: String,
    /// Optional Elasticsearch index name. Defaults to `argo-audit`
    /// inside the sink.
    pub index: String,
    /// Optional Datadog `service` field. Defaults to `argo-gateway`
    /// inside the sink.
    pub service: String,
    /// Poll interval, in seconds, between dispatcher sweeps. 0 ⇒
    /// the dispatcher's built-in default (15s).
    pub interval_secs: u64,
}
