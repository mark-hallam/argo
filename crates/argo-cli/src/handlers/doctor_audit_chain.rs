//! Audit-chain integrity probe for `argo doctor`.

use crate::home::db_path;
use argo_store::SessionStore;
use argo_types::SessionId;
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

pub(super) fn skipped() -> Value {
    json!({"status": "SKIP", "detail": "skip requested"})
}

pub(super) async fn collect(home: &Path) -> Value {
    let db = db_path(home);
    if !db.exists() {
        return json!({"status": "SKIP", "detail": "no sessions.db yet"});
    }
    let store = match SessionStore::open(&db).await {
        Ok(s) => s,
        Err(e) => return fail(format!("open store: {e}")),
    };
    if let Err(e) = store.migrate().await {
        return fail(format!("migrate: {e}"));
    }
    let sid = match newest_session(&store).await {
        Ok(Some(s)) => s,
        Ok(None) => return json!({"status": "OK", "detail": "no sessions yet", "head_seq": 0}),
        Err(e) => return fail(e),
    };
    let report = match argo_audit::verify_session(&store, sid).await {
        Ok(r) => r,
        Err(e) => return fail(format!("verify-chain: {e}")),
    };
    let head_seq = session_head_seq(&store, sid).await.unwrap_or_default();
    let break_at = report.breaks.iter().map(|b| b.seq).min();
    json!({"status": if report.ok { "OK" } else { "FAIL" },
        "detail": if report.ok { "chain verifiable" } else { "chain break detected" },
        "session_id": sid.to_string(), "head_seq": head_seq, "rows": report.rows,
        "break_at": break_at, "breaks": report.breaks.len()})
}

pub(super) fn print_text(v: &Value) {
    println!("  Audit-chain integrity:");
    println!(
        "    {:<4} {} head_seq={}",
        v["status"].as_str().unwrap_or("WARN"),
        v["detail"].as_str().unwrap_or(""),
        v["head_seq"].as_i64().unwrap_or(0)
    );
}

async fn newest_session(store: &SessionStore) -> Result<Option<SessionId>, String> {
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM session ORDER BY COALESCE(ended_at, started_at) DESC LIMIT 1",
    )
    .fetch_optional(store.pool())
    .await
    .map_err(|e| format!("newest session: {e}"))?;
    row.map(|b| Uuid::from_slice(&b).map(SessionId))
        .transpose()
        .map_err(|e| format!("session id: {e}"))
}

async fn session_head_seq(store: &SessionStore, sid: SessionId) -> Result<i64, String> {
    sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM audit_record WHERE session_id = ?")
        .bind(sid.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| format!("head seq: {e}"))
}

fn fail(detail: impl Into<String>) -> Value {
    json!({"status": "FAIL", "detail": detail.into()})
}
