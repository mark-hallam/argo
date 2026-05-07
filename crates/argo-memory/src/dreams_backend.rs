//! Dreams API backend for the memory summary tier
//! (`claude-dreams-memory-summarize`, V2.0 Integration A).
//!
//! Replaces the existing `argo-memory::summary` LLM-call path with
//! a [Claude Dreams API](https://platform.claude.com/docs/en/managed-agents/dreams)
//! call. Dreams is purpose-built for background reflection over a
//! worker's memory (≤100 sessions) producing curated outputs;
//! roughly the same cost as a Haiku summary but better quality and
//! non-destructive (the original memory rows are unchanged).
//!
//! Compiled in only when the crate's `dreams-backend` feature is
//! enabled. The legacy `summary.rs` path remains the default. The
//! brief's mission claims:
//!
//! - **Reliability prerequisite**: do not dream for a worker with
//!   open anomaly events ([`anomaly_guard`]). A misbehaving worker
//!   would poison its own dream.
//! - **Cost guardrail**: per-worker daily cap, default $5,
//!   overridable via `ARGO_DREAMS_DAILY_CAP_USD_MICROS`
//!   ([`DailyCapTracker`]).
//! - **Non-destructive**: Dreams results land as a new memory row
//!   tagged `kind=dream_summary, source=dreams_api` in the
//!   [`MemoryLayer::Summary`] layer.
//!
//! On-demand: `argo memory dream-now <worker>` (see
//! `argo-cli/src/handlers/memory_dream_now.rs`).
use crate::layers::MemoryLayer;
use crate::row::scope_as_str;
use crate::summary::YearMonth;
use crate::{MemoryEntry, MemoryFile, MemoryStore};
use argo_identity::MemoryNs;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default per-worker daily cap, expressed in USD micros ($5.00).
pub const DEFAULT_DAILY_CAP_USD_MICROS: i64 = 5_000_000;

/// Env var override for [`DEFAULT_DAILY_CAP_USD_MICROS`]. Value is
/// parsed as a base-10 i64 of USD micros.
pub const ENV_DAILY_CAP: &str = "ARGO_DREAMS_DAILY_CAP_USD_MICROS";

/// Env var override for [`DEFAULT_DREAMS_URL`].
pub const ENV_DREAMS_URL: &str = "ANTHROPIC_DREAMS_URL";

/// Default Dreams API endpoint.
pub const DEFAULT_DREAMS_URL: &str = "https://api.anthropic.com/v1/dreams";

/// Default per-call cost estimate when the response omits one
/// (matches the brief's $0.75/dream order-of-magnitude).
pub const DEFAULT_COST_USD_MICROS: i64 = 750_000;

/// Pluggable Dreams caller — production uses HTTP, tests use a
/// stub that returns a canned [`DreamResult`].
#[async_trait]
pub trait DreamsCaller: Send + Sync {
    /// Submit a memory-summary dream request for one namespace.
    async fn dream(
        &self,
        ns: &MemoryNs,
        month: YearMonth,
        rows: &[MemoryEntry],
    ) -> ArgoResult<DreamResult>;
}

/// Result of one Dreams API call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamResult {
    /// Polished summary body for the memory row.
    pub summary: String,
    /// Approximate cost in USD micros — feeds the daily cap.
    pub cost_micros: i64,
    /// Model identifier used by the Dreams backend.
    pub model: String,
}

/// HTTP-backed Dreams caller using `ANTHROPIC_API_KEY`. The endpoint
/// is overridable via `ANTHROPIC_DREAMS_URL` (useful for fixtures
/// or self-hosted proxies).
#[derive(Debug, Clone)]
pub struct EnvDreamsCaller {
    api_key: String,
    url: String,
    client: reqwest::Client,
}

impl EnvDreamsCaller {
    /// Build a caller from environment.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] when `ANTHROPIC_API_KEY` is
    /// missing.
    pub fn from_env() -> ArgoResult<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ArgoError::Provider("ANTHROPIC_API_KEY not set".into()))?;
        let url = std::env::var(ENV_DREAMS_URL).unwrap_or_else(|_| DEFAULT_DREAMS_URL.into());
        Ok(Self {
            api_key,
            url,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl DreamsCaller for EnvDreamsCaller {
    async fn dream(
        &self,
        ns: &MemoryNs,
        month: YearMonth,
        rows: &[MemoryEntry],
    ) -> ArgoResult<DreamResult> {
        let body = build_request_body(ns, month, rows);
        let resp = self
            .client
            .post(&self.url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("dreams send: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(ArgoError::Provider(format!("dreams http {status}: {txt}")));
        }
        let parsed: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("dreams decode: {e}")))?;
        parse_response(&parsed)
    }
}

fn build_request_body(
    ns: &MemoryNs,
    month: YearMonth,
    rows: &[MemoryEntry],
) -> serde_json::Value {
    serde_json::json!({
        "model": "claude-dreams-1",
        "scope": "memory_summary",
        "namespace": {
            "tenant_id": ns.tenant_id.0.to_string(),
            "worker_id": ns.worker_id.0.to_string(),
            "memory_scope": scope_as_str(ns.scope),
        },
        "month": month.label(),
        "rows": rows
            .iter()
            .map(|r| serde_json::json!({
                "id": r.id.to_string(),
                "body": r.body,
                "learned_at_micros": r.learned_at.timestamp_micros(),
            }))
            .collect::<Vec<_>>(),
    })
}

fn parse_response(parsed: &serde_json::Value) -> ArgoResult<DreamResult> {
    let summary = parsed
        .pointer("/dream/summary")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArgoError::Provider("dreams response missing /dream/summary".into()))?
        .to_string();
    let cost_micros = parsed
        .pointer("/dream/cost_usd_micros")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_COST_USD_MICROS);
    let model = parsed
        .pointer("/dream/model")
        .and_then(|v| v.as_str())
        .unwrap_or("claude-dreams-1")
        .to_string();
    Ok(DreamResult {
        summary,
        cost_micros,
        model,
    })
}

/// Per-(worker, day) cost cap. State lives at
/// `<state_root>/.skycot/dreams/<worker_id>/<YYYY-MM-DD>.json`,
/// where `state_root` is typically the resolved `~/.argo` home.
#[derive(Debug, Clone)]
pub struct DailyCapTracker {
    /// Directory rooted at `<state_root>/.skycot/dreams/<worker>`.
    pub state_dir: PathBuf,
    /// Cap for the worker, in USD micros. Default
    /// [`DEFAULT_DAILY_CAP_USD_MICROS`]; env override
    /// [`ENV_DAILY_CAP`].
    pub cap_micros: i64,
}

impl DailyCapTracker {
    /// Build a tracker rooted at `<state_root>/.skycot/dreams/<worker_id>/`.
    /// Reads the env-overridable cap from [`ENV_DAILY_CAP`] at
    /// construction time.
    #[must_use]
    pub fn new(state_root: &Path, worker_id: &str) -> Self {
        let cap_micros = std::env::var(ENV_DAILY_CAP)
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(DEFAULT_DAILY_CAP_USD_MICROS);
        let state_dir = state_root.join(".skycot/dreams").join(worker_id);
        Self {
            state_dir,
            cap_micros,
        }
    }

    /// Charge `charge_micros` against today's cumulative spend.
    ///
    /// Returns the new cumulative total on success.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] with a `dreams daily cap
    /// exceeded` prefix when the new total would exceed the cap.
    /// Returns [`ArgoError::Provider`] for any I/O error reading
    /// or writing the daily-state file.
    pub fn check_and_charge(&self, day: NaiveDate, charge_micros: i64) -> ArgoResult<i64> {
        std::fs::create_dir_all(&self.state_dir)
            .map_err(|e| ArgoError::Provider(format!("dreams cap dir: {e}")))?;
        let path = self.state_dir.join(format!("{}.json", day.format("%Y-%m-%d")));
        let mut spent: i64 = if path.exists() {
            let body = std::fs::read_to_string(&path)
                .map_err(|e| ArgoError::Provider(format!("dreams cap read: {e}")))?;
            serde_json::from_str::<i64>(body.trim())
                .map_err(|e| ArgoError::Provider(format!("dreams cap parse: {e}")))?
        } else {
            0
        };
        let next = spent.saturating_add(charge_micros);
        if next > self.cap_micros {
            return Err(ArgoError::Provider(format!(
                "dreams daily cap exceeded: spent={spent} charge={charge_micros} cap={cap}",
                cap = self.cap_micros
            )));
        }
        spent = next;
        std::fs::write(&path, serde_json::to_string(&spent).unwrap_or_default())
            .map_err(|e| ArgoError::Provider(format!("dreams cap write: {e}")))?;
        Ok(spent)
    }
}

/// Anomaly guard — refuse Dreams for workers with open anomaly
/// events (#847). A misbehaving worker would poison its own dream.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when one or more recent anomaly
/// events exist for `worker_id`.
pub async fn anomaly_guard(pool: &sqlx::SqlitePool, worker_id: &str) -> ArgoResult<()> {
    let events = argo_store::anomaly::list_for_worker(pool, worker_id, None, Some(1)).await?;
    if !events.is_empty() {
        return Err(ArgoError::Provider(format!(
            "dreams anomaly guard: {} open anomaly event(s) for worker {worker_id}; refusing to dream until resolved",
            events.len()
        )));
    }
    Ok(())
}

/// Marker prefix for a Dreams-summary memory row. Distinct from the
/// Haiku-summary marker so the two paths never collide.
#[must_use]
pub fn dream_summary_marker(ns: &MemoryNs, month: YearMonth, model: &str) -> String {
    format!(
        "<!-- argo-memory-dream-summary worker={} scope={} month={} model={} source=dreams_api -->",
        ns.worker_id,
        scope_as_str(ns.scope),
        month.label(),
        model
    )
}

/// Run a Dream for one worker namespace. Walks recent memory rows
/// for the previous calendar month, calls the Dreams API, then
/// persists the curated summary as a new memory row in the Summary
/// layer.
///
/// Order of guards:
///
/// 1. Anomaly guard (refuse if the worker has any open anomaly).
/// 2. Recent rows (no rows ⇒ nothing to dream over).
/// 3. Dreams API call.
/// 4. Daily cap (charged AFTER the call so the operator never
///    pays for a refused dream — but the cap still bounds the
///    NEXT one).
/// 5. Append to memory.
///
/// # Errors
/// Returns [`ArgoError`] on guard failure, no rows, transport, or
/// cap-exceeded conditions.
pub async fn run_dream_for_worker(
    memory: &MemoryStore,
    workspace_id: WorkspaceId,
    state_root: &Path,
    ns: &MemoryNs,
    now: DateTime<Utc>,
    caller: &dyn DreamsCaller,
) -> ArgoResult<MemoryEntry> {
    memory.ensure_layer_schema().await?;
    let pool = memory.pool();
    anomaly_guard(pool, &ns.worker_id.0.to_string()).await?;

    let month = YearMonth::previous(now);
    let start = month
        .start()
        .map_err(|e| ArgoError::Provider(format!("dreams month start: {e}")))?
        .timestamp_micros();
    let end = month
        .end()
        .map_err(|e| ArgoError::Provider(format!("dreams month end: {e}")))?
        .timestamp_micros();

    let rows: Vec<MemoryEntry> = memory
        .layered_entries(ns, MemoryLayer::Recent)
        .await?
        .into_iter()
        .map(|r| r.entry)
        .filter(|r| {
            let micros = r.learned_at.timestamp_micros();
            micros >= start && micros < end
        })
        .collect();
    if rows.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no recent memory rows to dream over for worker {} in {}",
            ns.worker_id,
            month.label()
        )));
    }

    let result = caller.dream(ns, month, &rows).await?;

    let cap = DailyCapTracker::new(state_root, &ns.worker_id.0.to_string());
    cap.check_and_charge(now.date_naive(), result.cost_micros)?;

    let marker = dream_summary_marker(ns, month, &result.model);
    let body = format!("{marker}\n{}", result.summary);
    memory
        .append_in_namespace_with_layer(
            ns,
            workspace_id,
            MemoryFile::Memory,
            body,
            None,
            MemoryLayer::Summary,
        )
        .await
}

#[cfg(test)]
#[path = "dreams_backend_tests.rs"]
mod tests;
