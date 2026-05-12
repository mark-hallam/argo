//! Out-of-band reaper for leaked E2B sandboxes. Lives next to
//! [`crate::e2b`] to keep the per-file LOC budget under control
//! while sharing the same [`E2bCall`] trait surface.
//!
//! The in-band cleanup path (CU executor's `Drop` / explicit close)
//! is unchanged. This reaper only matters when a session crashed
//! mid-run, the operator killed the worker before
//! `close_sandbox` could fire, or another out-of-band condition
//! leaked a sandbox handle. E2B keeps billing for the leaked
//! instance until its template's `pause_timeout` elapses; the
//! reaper closes idle sandboxes explicitly so operators pay only
//! for the configured threshold rather than the template default
//! (typically 5–15 min).
//!
//! ## Tenancy warning
//!
//! The E2B API-key is the tenancy boundary. Operators sharing one
//! key across multiple Argo workspaces (or with other E2B-using
//! applications) WILL reap each other's sandboxes. There is no
//! workspace-level filter — E2B doesn't tag sandboxes with
//! workspace identity, and reading every sandbox's metadata to
//! filter post-hoc is a different API surface. Document this in
//! operator runbooks; v2 may add a `--workspace-filter` once E2B
//! exposes per-sandbox metadata.

use crate::e2b::{E2bClient, E2bSandbox};
use crate::CuError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One row from E2B's `GET /sandboxes` list endpoint. `last_activity_at`
/// is optional because E2B may omit it for sandboxes that have
/// never seen activity since open; callers fall back to
/// `started_at` for the idle-time computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2bSandboxInfo {
    /// E2B-stamped sandbox id (UUID-like).
    #[serde(rename = "sandboxId", alias = "id")]
    pub id: String,
    /// Which template the sandbox was opened from.
    #[serde(rename = "templateId", alias = "template", default)]
    pub template: String,
    /// When the sandbox was opened.
    #[serde(rename = "startedAt", alias = "started_at")]
    pub started_at: DateTime<Utc>,
    /// Last recorded activity. `None` if E2B has never observed
    /// activity since open — callers fall back to `started_at`.
    #[serde(rename = "lastActivityAt", alias = "last_activity_at", default)]
    pub last_activity_at: Option<DateTime<Utc>>,
    /// Free-form status string from E2B (`running`, `paused`, etc.).
    #[serde(default)]
    pub status: String,
}

impl E2bSandboxInfo {
    /// Compute seconds idle. Falls back to `started_at` when
    /// `last_activity_at` is missing — semantically "no activity
    /// ever recorded" is treated as idle since open.
    #[must_use]
    pub fn idle_seconds(&self, now: DateTime<Utc>) -> i64 {
        let reference = self.last_activity_at.unwrap_or(self.started_at);
        (now - reference).num_seconds()
    }
}

/// Summary of one reap run. `scanned` is total sandboxes listed,
/// `reaped` is the ids of sandboxes the reaper closed successfully,
/// `errors` carries `(id, message)` pairs for any sandbox whose
/// `close_sandbox` call failed. Reaping continues past per-sandbox
/// errors rather than short-circuiting — operators get partial
/// progress on transient E2B failures.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2bReapReport {
    /// Total sandboxes inspected.
    pub scanned: usize,
    /// Ids of sandboxes successfully closed.
    pub reaped: Vec<String>,
    /// Per-sandbox close failures (`id`, error message).
    pub errors: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct ListSandboxesResponse {
    sandboxes: Vec<E2bSandboxInfo>,
    #[serde(default, rename = "nextCursor", alias = "next_cursor")]
    next_cursor: Option<String>,
}

impl E2bClient {
    /// List all sandboxes the configured API-key owns. Follows
    /// E2B's cursor-based pagination — operators with 100+ live
    /// sandboxes see multiple pages. Returns all pages concatenated.
    pub async fn list_sandboxes(&self) -> Result<Vec<E2bSandboxInfo>, CuError> {
        let mut cursor: Option<String> = None;
        let mut out: Vec<E2bSandboxInfo> = Vec::new();
        loop {
            let mut url = format!("{}/sandboxes", self.base_url);
            if let Some(c) = cursor.as_deref() {
                url.push_str(&format!("?cursor={c}"));
            }
            let resp = self
                .http
                .get(&url)
                .bearer_auth(&self.api_key)
                .send()
                .await
                .map_err(|e| CuError::Http {
                    stage: "e2b/sandbox.list",
                    source: e,
                })?;
            let status = resp.status();
            if !status.is_success() {
                let body = match resp.text().await {
                    Ok(mut t) => {
                        if t.len() > 512 {
                            t.truncate(512);
                            t.push_str("…(truncated)");
                        }
                        t
                    }
                    Err(_) => "(body read failed)".into(),
                };
                return Err(CuError::HttpStatus {
                    stage: "e2b/sandbox.list",
                    status: status.as_u16(),
                    body,
                });
            }
            let parsed: ListSandboxesResponse =
                resp.json().await.map_err(|e| CuError::Http {
                    stage: "e2b/sandbox.list.parse",
                    source: e,
                })?;
            out.extend(parsed.sandboxes);
            match parsed.next_cursor {
                Some(c) if !c.is_empty() => cursor = Some(c),
                _ => break,
            }
        }
        Ok(out)
    }
}

/// Filter `sandboxes` to those whose idle time strictly exceeds
/// `max_idle_secs`. Pure helper extracted from `reap_idle` so the
/// trait's default implementation can stay small AND unit tests
/// can exercise the threshold boundary without an `E2bCall` mock.
///
/// Boundary semantics: "exactly at threshold" is NOT reaped. A
/// sandbox idle for exactly `max_idle_secs` survives; one idle
/// for `max_idle_secs + 1` is reaped.
#[must_use]
pub fn filter_idle(
    sandboxes: &[E2bSandboxInfo],
    max_idle_secs: u64,
    now: DateTime<Utc>,
) -> Vec<E2bSandboxInfo> {
    let threshold = i64::try_from(max_idle_secs).unwrap_or(i64::MAX);
    sandboxes
        .iter()
        .filter(|s| s.idle_seconds(now) > threshold)
        .cloned()
        .collect()
}

/// Wrap a bare sandbox id in the shared [`E2bSandbox`] handle so the
/// reaper can call [`crate::e2b::E2bCall::close_sandbox`] without
/// constructing the type out-of-module.
#[must_use]
pub fn sandbox_handle(id: &str) -> E2bSandbox {
    E2bSandbox { id: id.to_owned() }
}

#[cfg(test)]
#[path = "e2b_reap_tests.rs"]
mod tests;
