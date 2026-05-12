//! `system:audit.evidence_archival` — daily 2am UTC audit-archival tick.
//!
//! Wraps [`argo_audit::archive::archive_window`] against the
//! workspace SQLite (`<workspace>/sessions.db`) + the default
//! archive directory (`<workspace>/audit-archive/`). Idempotent
//! within a UTC day via `archive_window`'s built-in manifest check.
//! Pruning is off by default; operators flip it on via the standalone
//! `argo audit archive --prune` CLI after verifying bundles.
use argo_audit::archive::{archive_window, ArchiveConfig, DEFAULT_RETENTION_DAYS};
use argo_cron::{SystemJobContext, SystemJobHandler, SystemJobOutcome};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Canonical system-job name. Matches `cron/system/audit_evidence_archival_tick.json::target.chat`.
pub const SYSTEM_JOB_AUDIT_EVIDENCE_ARCHIVAL: &str = "audit.evidence_archival";

/// Pluggable seam over "run one archive tick against `workspace_root`".
/// The default impl opens `<workspace>/sessions.db` and writes bundles
/// into `<workspace>/audit-archive/`; tests use [`NoopArchiver`].
#[async_trait]
pub trait AuditArchiver: Send + Sync {
    /// Execute one archival pass. Returns (row_count, bundle_filename).
    async fn archive_one(&self, workspace_root: &Path) -> ArgoResult<(u64, String)>;
}

/// Production archiver: opens the workspace SQLite store and writes
/// a bundle under `<workspace>/audit-archive/` with default retention.
pub struct WorkspaceArchiver;

#[async_trait]
impl AuditArchiver for WorkspaceArchiver {
    async fn archive_one(&self, workspace_root: &Path) -> ArgoResult<(u64, String)> {
        let db = workspace_root.join("sessions.db");
        let store = SessionStore::open(&db).await?;
        store.migrate().await?;
        let mut cfg = ArchiveConfig::new(workspace_root.join("audit-archive"));
        cfg.retention_days = DEFAULT_RETENTION_DAYS;
        let bundle = archive_window(&store, &cfg).await?;
        Ok((bundle.row_count, bundle.filename))
    }
}

/// No-op archiver returning `(0, "")` — exposed for tests and for
/// downstream callers that want to short-circuit the cron tick.
#[allow(dead_code)]
pub struct NoopArchiver;

#[async_trait]
impl AuditArchiver for NoopArchiver {
    async fn archive_one(&self, _workspace_root: &Path) -> ArgoResult<(u64, String)> {
        Ok((0, String::new()))
    }
}

/// Audit-archival tick handler.
pub struct AuditArchivalHandler {
    archiver: Arc<dyn AuditArchiver>,
}

impl AuditArchivalHandler {
    /// Build with the supplied archiver.
    #[must_use]
    pub fn new(archiver: Arc<dyn AuditArchiver>) -> Self {
        Self { archiver }
    }
}

#[async_trait]
impl SystemJobHandler for AuditArchivalHandler {
    async fn run(&self, ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
        let root: PathBuf = ctx.workspace_root.clone();
        match self.archiver.archive_one(&root).await {
            Ok((_, name)) if name.is_empty() => Ok(SystemJobOutcome::empty(
                "audit.evidence_archival: noop archiver (not wired)",
            )),
            Ok((rows, name)) => Ok(SystemJobOutcome {
                jobs_processed: rows as usize,
                skipped: 0,
                errors: Vec::new(),
                summary: format!(
                    "audit.evidence_archival: archived {rows} row(s) → {name}"
                ),
            }),
            Err(ArgoError::Provider(msg)) => Ok(SystemJobOutcome {
                jobs_processed: 0,
                skipped: 0,
                errors: vec![msg.clone()],
                summary: format!("audit.evidence_archival: failed — {msg}"),
            }),
            Err(e) => Err(e),
        }
    }
}

/// Build a handler wired to [`WorkspaceArchiver`] — the default
/// production wiring once `argo cron init-system` registers the
/// template. Idempotency is delegated to `archive_window`.
#[must_use]
pub fn default_handler() -> Arc<dyn SystemJobHandler> {
    Arc::new(AuditArchivalHandler::new(Arc::new(WorkspaceArchiver)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_cron::SystemJobContext;

    #[tokio::test]
    async fn noop_archiver_emits_empty_outcome() {
        let h = AuditArchivalHandler::new(Arc::new(NoopArchiver));
        let ctx = SystemJobContext::at_now(PathBuf::from("/tmp"));
        let out = h.run(ctx).await.unwrap();
        assert_eq!(out.jobs_processed, 0);
        assert!(out.summary.contains("noop"));
    }

    struct CountingArchiver(u64);
    #[async_trait]
    impl AuditArchiver for CountingArchiver {
        async fn archive_one(&self, _w: &Path) -> ArgoResult<(u64, String)> {
            Ok((self.0, format!("{}-bundle.tar.gz", self.0)))
        }
    }

    #[tokio::test]
    async fn live_archiver_reports_row_count() {
        let h = AuditArchivalHandler::new(Arc::new(CountingArchiver(42)));
        let ctx = SystemJobContext::at_now(PathBuf::from("/tmp"));
        let out = h.run(ctx).await.unwrap();
        assert_eq!(out.jobs_processed, 42);
        assert!(out.summary.contains("42 row(s)"));
        assert!(out.summary.contains("42-bundle.tar.gz"));
    }
}
