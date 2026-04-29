//! `argo audit verify` chain verifier.
//!
//! Walks the `audit_record` rows for a session in `seq` order,
//! recomputes `self_hash` from `(parent_hash, kind, payload_json)`,
//! and asserts every row matches.
use crate::event::AuditKind;
use crate::sink::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::collections::HashMap;
use std::path::PathBuf;

/// One break detected by the verifier.
#[derive(Debug, Clone)]
pub struct ChainBreak {
    /// `seq` of the broken row.
    pub seq: i64,
    /// `kind` column value.
    pub kind: String,
    /// Free-form description of what broke (parent mismatch, hash
    /// mismatch, unknown kind, …).
    pub reason: String,
}

/// Result of verifying one session's audit chain.
#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    /// Number of rows examined.
    pub rows: usize,
    /// Whether the chain verifies cleanly.
    pub ok: bool,
    /// Any breaks (empty when `ok = true`).
    pub breaks: Vec<ChainBreak>,
    /// Histogram of `kind → count`.
    pub kind_counts: HashMap<String, usize>,
    /// Optional per-session fixture path the verifier was run against
    /// (used by the CLI's `--fixture` flag, unused here in-process).
    pub fixture_path: Option<PathBuf>,
}

/// Verify the full audit chain for `session_id`.
pub async fn verify_session(
    store: &SessionStore,
    session_id: SessionId,
) -> ArgoResult<VerifyReport> {
    let rows: Vec<RawRow> = sqlx::query_as::<_, RawRow>(
        "SELECT seq, kind, payload_json, parent_hash, self_hash \
         FROM audit_record WHERE session_id = ? ORDER BY seq ASC",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit verify fetch: {e}")))?;
    let report = verify_rows(rows);
    Ok(report)
}

/// Verify the full audit chain across every session in the store.
/// Each session's chain is verified independently — chain heads do
/// not connect across sessions.
/// Used for the §17.2 "10k-record corpus" gate.
pub async fn verify_all(store: &SessionStore) -> ArgoResult<VerifyReport> {
    let session_ids: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT DISTINCT session_id FROM audit_record \
         WHERE session_id IS NOT NULL ORDER BY session_id ASC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit verify_all sessions: {e}")))?;
    let mut combined = VerifyReport {
        rows: 0,
        ok: true,
        ..Default::default()
    };
    for sid in session_ids {
        let rows: Vec<RawRow> = sqlx::query_as::<_, RawRow>(
            "SELECT seq, kind, payload_json, parent_hash, self_hash \
             FROM audit_record WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(&sid)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("audit verify_all fetch: {e}")))?;
        merge_into(&mut combined, verify_rows(rows));
    }
    // Plus the global (session_id IS NULL) chain.
    let null_rows: Vec<RawRow> = sqlx::query_as::<_, RawRow>(
        "SELECT seq, kind, payload_json, parent_hash, self_hash \
         FROM audit_record WHERE session_id IS NULL ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit verify_all null fetch: {e}")))?;
    merge_into(&mut combined, verify_rows(null_rows));
    Ok(combined)
}

fn merge_into(out: &mut VerifyReport, src: VerifyReport) {
    out.rows += src.rows;
    out.ok = out.ok && src.ok;
    out.breaks.extend(src.breaks);
    for (k, v) in src.kind_counts {
        *out.kind_counts.entry(k).or_insert(0) += v;
    }
}

#[derive(sqlx::FromRow)]
struct RawRow {
    seq: i64,
    kind: String,
    payload_json: String,
    parent_hash: Vec<u8>,
    self_hash: Vec<u8>,
}

fn verify_rows(rows: Vec<RawRow>) -> VerifyReport {
    let mut report = VerifyReport {
        rows: rows.len(),
        ok: true,
        ..Default::default()
    };
    let mut prev_self_hash: Option<[u8; 32]> = None;
    for row in rows {
        *report.kind_counts.entry(row.kind.clone()).or_insert(0) += 1;
        if row.parent_hash.len() != 32 || row.self_hash.len() != 32 {
            report.ok = false;
            report.breaks.push(ChainBreak {
                seq: row.seq,
                kind: row.kind.clone(),
                reason: "parent_hash or self_hash is not 32 bytes".into(),
            });
            continue;
        }
        let parent: [u8; 32] = row.parent_hash.as_slice().try_into().unwrap_or([0u8; 32]);
        let self_h: [u8; 32] = row.self_hash.as_slice().try_into().unwrap_or([0u8; 32]);
        if let Some(prev) = prev_self_hash {
            if prev != parent {
                report.ok = false;
                report.breaks.push(ChainBreak {
                    seq: row.seq,
                    kind: row.kind.clone(),
                    reason: "parent_hash does not match previous row's self_hash".into(),
                });
            }
        }
        let recomputed = compute_chain_hash(&parent, &row.kind, &row.payload_json);
        if recomputed != self_h {
            report.ok = false;
            report.breaks.push(ChainBreak {
                seq: row.seq,
                kind: row.kind.clone(),
                reason: "self_hash does not match recomputed chain hash".into(),
            });
        }
        if AuditKind::from_label(&row.kind).is_none() {
            report.ok = false;
            report.breaks.push(ChainBreak {
                seq: row.seq,
                kind: row.kind.clone(),
                reason: "unknown audit kind label".into(),
            });
        }
        prev_self_hash = Some(self_h);
    }
    report
}
