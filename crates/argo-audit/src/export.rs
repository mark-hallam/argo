//! NDJSON export of the audit log (§17.4).
//!
//! Streams rows from `audit_record` in `seq` order, one JSON object
//! per line. Each row carries the canonical fields plus `seq` and a
//! hex-encoded `self_hash` so external tools can re-run the chain
//! check without round-tripping through SQLite.
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

/// One exported row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedRow {
    /// Append-only sequence number (monotonic across the whole DB).
    pub seq: i64,
    /// UUIDv7 row id.
    pub id: String,
    /// Owning session, if any.
    pub session_id: Option<String>,
    /// Event kind label (e.g. `"turn_start"`).
    pub kind: String,
    /// Canonical payload as serde_json::Value.
    pub payload: serde_json::Value,
    /// Parent hash (hex).
    pub parent_hash: String,
    /// Self hash (hex).
    pub self_hash: String,
    /// `created_at` (UTC, epoch micros).
    pub created_at_micros: i64,
}

#[derive(sqlx::FromRow)]
struct Row {
    seq: i64,
    id: Vec<u8>,
    session_id: Option<Vec<u8>>,
    kind: String,
    payload_json: String,
    parent_hash: Vec<u8>,
    self_hash: Vec<u8>,
    created_at: i64,
}

/// Stream every row in `seq` order. The callback is invoked for
/// each row; returning an error from the callback aborts the
/// export and is propagated.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on SQL failure.
pub async fn export_ndjson<F>(store: &SessionStore, mut emit: F) -> ArgoResult<usize>
where
    F: FnMut(&ExportedRow) -> ArgoResult<()>,
{
    let rows = sqlx::query_as::<_, Row>(
        "SELECT seq, id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at FROM audit_record \
         ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit export query: {e}")))?;
    let mut count = 0_usize;
    for row in rows {
        let exported = row_to_exported(row)?;
        emit(&exported)?;
        count += 1;
    }
    Ok(count)
}

/// Fetch the most recent `n` rows in `seq` order (oldest first).
/// Used by the CLI's `argo audit show --tail N`.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on SQL or payload parse failure.
pub async fn tail_rows(store: &SessionStore, n: u32) -> ArgoResult<Vec<ExportedRow>> {
    tail_rows_filtered(store, n, None).await
}

/// Fetch the most recent `n` rows of an optional `kind`, in `seq`
/// order (oldest first). When `kind` is `None`, behaves identically
/// to [`tail_rows`]; when `Some`, the WHERE-clause is applied
/// before the LIMIT so the caller gets up to `n` rows of the
/// requested kind. Used by the gateway's `GET /audit/recent`.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on SQL or payload parse failure.
pub async fn tail_rows_filtered(
    store: &SessionStore,
    n: u32,
    kind: Option<&str>,
) -> ArgoResult<Vec<ExportedRow>> {
    let rows = if let Some(k) = kind {
        sqlx::query_as::<_, Row>(
            "SELECT * FROM (
                 SELECT seq, id, session_id, kind, payload_json, \
                        parent_hash, self_hash, created_at \
                 FROM audit_record WHERE kind = ? \
                 ORDER BY seq DESC LIMIT ?
             ) ORDER BY seq ASC",
        )
        .bind(k)
        .bind(i64::from(n))
        .fetch_all(store.pool())
        .await
    } else {
        sqlx::query_as::<_, Row>(
            "SELECT * FROM (
                 SELECT seq, id, session_id, kind, payload_json, \
                        parent_hash, self_hash, created_at \
                 FROM audit_record ORDER BY seq DESC LIMIT ?
             ) ORDER BY seq ASC",
        )
        .bind(i64::from(n))
        .fetch_all(store.pool())
        .await
    }
    .map_err(|e| ArgoError::Provider(format!("audit tail query: {e}")))?;
    rows.into_iter().map(row_to_exported).collect()
}

/// Fetch every audit row for one session in `seq` order.
/// Used by `argo audit show --session <id>` for full per-session
/// timeline inspection. No LIMIT — sessions are typically small.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on SQL or payload parse failure.
pub async fn rows_for_session(
    store: &SessionStore,
    session_id: argo_types::SessionId,
) -> ArgoResult<Vec<ExportedRow>> {
    let rows = sqlx::query_as::<_, Row>(
        "SELECT seq, id, session_id, kind, payload_json, \
                parent_hash, self_hash, created_at \
         FROM audit_record WHERE session_id = ? ORDER BY seq ASC",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit session query: {e}")))?;
    rows.into_iter().map(row_to_exported).collect()
}

fn row_to_exported(row: Row) -> ArgoResult<ExportedRow> {
    Ok(ExportedRow {
        seq: row.seq,
        id: hex::encode(&row.id),
        session_id: row.session_id.map(|b| hex::encode(&b)),
        kind: row.kind,
        payload: serde_json::from_str(&row.payload_json)
            .map_err(|e| ArgoError::Provider(format!("audit payload parse: {e}")))?,
        parent_hash: hex::encode(&row.parent_hash),
        self_hash: hex::encode(&row.self_hash),
        created_at_micros: row.created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuditKind;
    use argo_store::SessionStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn export_round_trips_rows_in_seq_order() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let sink = crate::AuditSink::new(Arc::clone(&store));
        for i in 0..3 {
            sink.write(None, AuditKind::TurnStart, serde_json::json!({"i": i}))
                .await
                .unwrap();
        }
        let mut seqs = Vec::new();
        let n = export_ndjson(&store, |row| {
            seqs.push(row.seq);
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(n, 3);
        assert_eq!(seqs, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn tail_rows_filtered_returns_only_matching_kind() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let sink = crate::AuditSink::new(Arc::clone(&store));
        // 3 turn_start, 2 assistant_message — interleaved.
        for i in 0..3 {
            sink.write(None, AuditKind::TurnStart, serde_json::json!({"i": i}))
                .await
                .unwrap();
            if i < 2 {
                sink.write(
                    None,
                    AuditKind::AssistantMessage,
                    serde_json::json!({"i": i}),
                )
                .await
                .unwrap();
            }
        }
        // Unfiltered: take 2 most recent of any kind, in seq order.
        let unfiltered = tail_rows_filtered(&store, 2, None).await.unwrap();
        assert_eq!(unfiltered.len(), 2);
        // Filtered to turn_start: take 2 most recent turn_starts.
        let starts = tail_rows_filtered(&store, 2, Some("turn_start"))
            .await
            .unwrap();
        assert_eq!(starts.len(), 2);
        for r in &starts {
            assert_eq!(r.kind, "turn_start");
        }
        // Filtered to assistant_message: only 2 exist.
        let msgs = tail_rows_filtered(&store, 10, Some("assistant_message"))
            .await
            .unwrap();
        assert_eq!(msgs.len(), 2);
        for r in &msgs {
            assert_eq!(r.kind, "assistant_message");
        }
        // Bogus kind: empty result.
        let nope = tail_rows_filtered(&store, 10, Some("nope"))
            .await
            .unwrap();
        assert!(nope.is_empty());
    }

    #[tokio::test]
    async fn callback_error_aborts_export() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let sink = crate::AuditSink::new(Arc::clone(&store));
        for i in 0..5 {
            sink.write(None, AuditKind::TurnStart, serde_json::json!({"i": i}))
                .await
                .unwrap();
        }
        let mut seen = 0_usize;
        let result = export_ndjson(&store, |_row| {
            seen += 1;
            if seen == 2 {
                Err(ArgoError::Provider("synthetic abort".into()))
            } else {
                Ok(())
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(seen, 2);
    }
}
