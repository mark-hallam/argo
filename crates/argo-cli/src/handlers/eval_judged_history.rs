//! Shared read layer over a tenant's real *judged* history.
//!
//! Two CLI surfaces consume the same rows:
//!
//! - `argo eval export-training-data` → SFT-JSONL training data
//!   (`eval_export_training_data`), and
//! - `argo eval generate-cases` → `argo-eval` regression cases
//!   (`eval_generate_cases`, Step 1b).
//!
//! Keeping the queries here (rather than duplicating them per handler)
//! means the join over `approval_request` + `worker.retrain.committed`
//! audit rows is defined once. Read-only — **no new table, no
//! migration**. Both surfaces are tenant-scoped at the query boundary.
//! Design doc:
//! `docs/skycot/design/2026-05-29-gap-closure-plans/03-self-improvement-training-export.md`.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::Value;
use std::path::Path;

/// Audit endpoint marking a committed retrain (human correction).
pub const RETRAIN_COMMIT_ENDPOINT: &str = "worker.retrain.committed";

/// One decided approval request — a human judged a proposed action
/// good (`granted`) or bad (`denied`/expired). Source-neutral: the
/// export turns it into an SFT pair, the generator turns a *rejected*
/// one into a "must refuse or defer" regression case.
#[derive(Debug, Clone)]
pub struct DecidedApproval {
    /// The draft action the teammate proposed (free text; PII-redacted
    /// downstream at the serialization boundary by each consumer).
    pub draft: Option<String>,
    /// Raw status: `granted` / `denied` / `expired`.
    pub status: String,
    /// Decision rationale recorded by the human, if any.
    pub decision: Option<String>,
    /// Actor that requested the action.
    pub requested_by: String,
    /// Actor that decided it.
    pub decided_by: Option<String>,
}

impl DecidedApproval {
    /// `true` when the human refused the action (denied or expired) —
    /// i.e. a labelled "do NOT do this" the generator turns into a
    /// refuse-or-defer regression case.
    #[must_use]
    pub fn was_refused(&self) -> bool {
        self.status != "granted"
    }
}

/// One committed retrain (a human correction). Carries the
/// classification (`vocabulary_patch`, `action_ban`, …) plus the raw
/// `structural_change` payload, so consumers can render the correction
/// without re-querying.
#[derive(Debug, Clone)]
pub struct CommittedRetrain {
    /// Worker the correction was applied to.
    pub worker_id: Option<String>,
    /// Retrain classification (defaults to `memory_fact` when absent).
    pub classification: String,
    /// Raw `structural_change` object from the audit payload.
    pub structural_change: Option<Value>,
}

/// Open the live session store from the resolved home dir (mirrors the
/// other read-only CLI handlers).
pub async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

/// Parse a tenant UUID, mapping a bad value to a `Schema` error.
pub fn parse_tenant(tenant: &str) -> ArgoResult<TenantId> {
    uuid::Uuid::parse_str(tenant)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{tenant}': {e}")))
}

/// All decided (non-pending) approval requests for the tenant,
/// newest-first. `approval_request.tenant_id` is a BLOB column.
pub async fn decided_approvals(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<DecidedApproval>> {
    type Row = (Option<String>, String, Option<String>, String, Option<String>);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT draft_content, status, decision, requested_by, decided_by \
         FROM approval_request \
         WHERE tenant_id = ? AND status != 'pending' \
         ORDER BY decided_at DESC",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list decided approvals: {e}")))?;
    Ok(rows
        .into_iter()
        .map(
            |(draft, status, decision, requested_by, decided_by)| DecidedApproval {
                draft,
                status,
                decision,
                requested_by,
                decided_by,
            },
        )
        .collect())
}

/// All committed retrains for the tenant, newest-first. The audit
/// `tenant_id` is a TEXT column (unlike `approval_request`).
pub async fn committed_retrains(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<CommittedRetrain>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = ? \
         ORDER BY seq DESC",
    )
    .bind(tenant_id.to_string())
    .bind(RETRAIN_COMMIT_ENDPOINT)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list committed retrains: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for (payload,) in rows {
        let Ok(v) = serde_json::from_str::<Value>(&payload) else {
            continue;
        };
        let classification = v
            .get("classification")
            .and_then(Value::as_str)
            .unwrap_or("memory_fact")
            .to_string();
        let worker_id = v
            .get("worker_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        out.push(CommittedRetrain {
            worker_id,
            classification,
            structural_change: v.get("structural_change").cloned(),
        });
    }
    Ok(out)
}

/// Convenience for handlers: resolve the tenant, open the store.
pub async fn open_for_tenant(tenant: &str) -> ArgoResult<(SessionStore, TenantId)> {
    let tenant_id = parse_tenant(tenant)?;
    let store = open_store().await?;
    Ok((store, tenant_id))
}

/// PII-redact a free-text field, returning `(text, was_redacted)`.
/// Owned `Cow` ⇒ redaction was applied. Mirrors the audit sink.
/// `context` is the static redaction site label the detector records.
#[must_use]
pub fn redact(s: &str, context: &'static str) -> (String, bool) {
    use argo_security::pii_detect::scan_and_maybe_redact;
    use std::borrow::Cow;
    match scan_and_maybe_redact(s, context) {
        Cow::Borrowed(_) => (s.to_string(), false),
        Cow::Owned(red) => (red, true),
    }
}

/// Write JSONL lines to `out`, returning the number written.
pub fn write_lines(out: &Path, lines: &[String]) -> ArgoResult<usize> {
    use std::io::Write;
    let mut file = std::fs::File::create(out)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", out.display())))?;
    for line in lines {
        writeln!(file, "{line}")
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", out.display())))?;
    }
    Ok(lines.len())
}
