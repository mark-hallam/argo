//! Nightly memory summarisation.
use crate::layers::MemoryLayer;
use crate::row::{ns_from_parts, scope_as_str};
use crate::{MemoryEntry, MemoryFile, MemoryStore};
use argo_identity::MemoryNs;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Workspace-local cron schedule for the monthly memory summary job.
pub const NIGHTLY_SUMMARY_CRON: &str = "0 2 * * *";

/// Calendar month addressed by the summary job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct YearMonth {
    /// Four-digit year.
    pub year: i32,
    /// Month number, 1 through 12.
    pub month: u32,
}

impl YearMonth {
    /// Month immediately before `now`.
    #[must_use]
    pub fn previous(now: DateTime<Utc>) -> Self {
        let (year, month) = if now.month() == 1 {
            (now.year() - 1, 12)
        } else {
            (now.year(), now.month() - 1)
        };
        Self { year, month }
    }

    /// Stable `YYYY-MM` label.
    #[must_use]
    pub fn label(self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }

    fn start(self) -> ArgoResult<DateTime<Utc>> {
        Utc.with_ymd_and_hms(self.year, self.month, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| ArgoError::Schema(format!("invalid summary month {}", self.label())))
    }

    fn end(self) -> ArgoResult<DateTime<Utc>> {
        let (year, month) = if self.month == 12 {
            (self.year + 1, 1)
        } else {
            (self.year, self.month + 1)
        };
        Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| ArgoError::Schema(format!("invalid summary month {}", self.label())))
    }
}

/// Provider abstraction for monthly memory summaries.
pub trait MemorySummaryProvider: Send + Sync {
    /// Model identifier used to produce the summary.
    fn model(&self) -> &'static str;

    /// Summarise one worker namespace for a month.
    fn summarize(
        &self,
        ns: &MemoryNs,
        month: YearMonth,
        rows: &[MemoryEntry],
    ) -> ArgoResult<String>;
}

/// Haiku summary adapter for the nightly job.
#[derive(Debug, Default, Clone, Copy)]
pub struct HaikuSummaryProvider;

impl MemorySummaryProvider for HaikuSummaryProvider {
    fn model(&self) -> &'static str {
        "claude-haiku-4-5"
    }

    fn summarize(
        &self,
        ns: &MemoryNs,
        month: YearMonth,
        rows: &[MemoryEntry],
    ) -> ArgoResult<String> {
        let mut body = format!(
            "Summary for worker {} in {} ({} rows, model {}).",
            ns.worker_id,
            month.label(),
            rows.len(),
            self.model()
        );
        for row in rows.iter().take(12) {
            body.push_str("\n- ");
            body.push_str(row.body.trim());
        }
        Ok(body)
    }
}

/// Result of one nightly summary run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthlySummaryReport {
    /// Month summarised.
    pub month: YearMonth,
    /// Worker namespaces found with recent rows.
    pub workers_seen: usize,
    /// New summary rows written.
    pub summaries_written: usize,
}

impl MemoryStore {
    /// Run the 2am workspace-local monthly summary job.
    pub async fn run_nightly_summary_job(
        &self,
        workspace_id: WorkspaceId,
        now: DateTime<Utc>,
        provider: &dyn MemorySummaryProvider,
    ) -> ArgoResult<MonthlySummaryReport> {
        self.ensure_layer_schema().await?;
        let month = YearMonth::previous(now);
        let start = month.start()?.timestamp_micros();
        let end = month.end()?.timestamp_micros();
        let namespaces = self.summary_namespaces(workspace_id, start, end).await?;
        let mut written = 0;
        for ns in &namespaces {
            let rows = self.recent_rows_for_month(ns, start, end).await?;
            if rows.is_empty()
                || self
                    .summary_exists(workspace_id, ns, month, provider.model())
                    .await?
            {
                continue;
            }
            let marker = summary_marker(ns, month, provider.model());
            let body = format!("{marker}\n{}", provider.summarize(ns, month, &rows)?);
            self.append_in_namespace_with_layer(
                ns,
                workspace_id,
                MemoryFile::Memory,
                body,
                None,
                MemoryLayer::Summary,
            )
            .await?;
            written += 1;
        }
        Ok(MonthlySummaryReport {
            month,
            workers_seen: namespaces.len(),
            summaries_written: written,
        })
    }

    async fn summary_namespaces(
        &self,
        workspace_id: WorkspaceId,
        start: i64,
        end: i64,
    ) -> ArgoResult<Vec<MemoryNs>> {
        let rows = sqlx::query(
            "SELECT tenant_id, worker_id, memory_scope \
             FROM memory_entry \
             WHERE workspace_id = ? AND layer = 'recent' \
               AND tenant_id IS NOT NULL AND worker_id IS NOT NULL \
               AND memory_scope IS NOT NULL \
               AND learned_at >= ? AND learned_at < ? \
             GROUP BY tenant_id, worker_id, memory_scope",
        )
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(start)
        .bind(end)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory summary namespaces: {e}")))?;
        rows.into_iter()
            .map(|row| {
                let tenant_id: Vec<u8> = row
                    .try_get("tenant_id")
                    .map_err(|e| ArgoError::Provider(format!("memory summary tenant_id: {e}")))?;
                let worker_id: Vec<u8> = row
                    .try_get("worker_id")
                    .map_err(|e| ArgoError::Provider(format!("memory summary worker_id: {e}")))?;
                let scope: String = row
                    .try_get("memory_scope")
                    .map_err(|e| ArgoError::Provider(format!("memory summary scope: {e}")))?;
                ns_from_parts(&tenant_id, &worker_id, &scope)
            })
            .collect()
    }

    async fn recent_rows_for_month(
        &self,
        ns: &MemoryNs,
        start: i64,
        end: i64,
    ) -> ArgoResult<Vec<MemoryEntry>> {
        Ok(self
            .layered_entries(ns, MemoryLayer::Recent)
            .await?
            .into_iter()
            .map(|row| row.entry)
            .filter(|row| {
                let learned_at = row.learned_at.timestamp_micros();
                learned_at >= start && learned_at < end
            })
            .collect())
    }

    async fn summary_exists(
        &self,
        workspace_id: WorkspaceId,
        ns: &MemoryNs,
        month: YearMonth,
        model: &str,
    ) -> ArgoResult<bool> {
        let pattern = format!("{}%", summary_marker(ns, month, model));
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM memory_entry \
             WHERE workspace_id = ? AND tenant_id = ? AND worker_id = ? \
               AND memory_scope = ? AND layer = 'summary' \
               AND superseded_by IS NULL AND body LIKE ? LIMIT 1",
        )
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(ns.tenant_id.0.as_bytes().as_slice())
        .bind(ns.worker_id.0.as_bytes().as_slice())
        .bind(scope_as_str(ns.scope))
        .bind(pattern)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory summary exists: {e}")))?;
        Ok(exists.is_some())
    }
}

fn summary_marker(ns: &MemoryNs, month: YearMonth, model: &str) -> String {
    format!(
        "<!-- argo-memory-summary worker={} scope={} month={} model={} -->",
        ns.worker_id,
        scope_as_str(ns.scope),
        month.label(),
        model
    )
}
