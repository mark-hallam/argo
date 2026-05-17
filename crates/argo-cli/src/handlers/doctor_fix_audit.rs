//! Hash-chained `audit_record` append for completed `argo doctor fix` actions.

use super::doctor_fix::{FixContext, FixReport, AUDIT_KIND};
use crate::home::db_path;
use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

pub(super) async fn append_audit(
    ctx: &FixContext,
    report: &FixReport,
    auto: bool,
) -> ArgoResult<()> {
    std::fs::create_dir_all(&ctx.home)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", ctx.home.display())))?;
    let store = SessionStore::open(db_path(&ctx.home)).await?;
    store.migrate().await?;
    let parent = last_global_hash(&store).await?;
    let payload = json!({"issue_id": report.id, "category": report.category, "status": report.status, "detail": report.detail, "path": report.path, "remediation": report.remediation, "auto": auto});
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("doctor fix audit JSON: {e}")))?;
    let self_hash = compute_chain_hash(&parent, AUDIT_KIND, &canonical);
    sqlx::query("INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) VALUES (?, NULL, ?, ?, ?, ?, ?)")
        .bind(Uuid::now_v7().as_bytes().as_slice())
        .bind(AUDIT_KIND)
        .bind(canonical)
        .bind(parent.as_slice())
        .bind(self_hash.as_slice())
        .bind(Utc::now().timestamp_micros())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("doctor fix audit: {e}")))?;
    Ok(())
}

async fn last_global_hash(store: &SessionStore) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("doctor fix last audit hash: {e}")))?;
    let mut out = [0_u8; 32];
    if let Some(bytes) = bytes {
        if bytes.len() == 32 {
            out.copy_from_slice(&bytes);
        }
    }
    Ok(out)
}
