//! `argo audit verify` chain verifier.
//!
//! Walks the `audit_record` rows for a session in `seq` order,
//! recomputes `self_hash` from `(parent_hash, kind, payload_json)`,
//! and asserts every row matches.
//!
//! As of Stream `mission-enterprise-audit-full-text-search`
//! (migration v67), the verifier also asserts the 7 denormalized
//! filter columns (worker / channel / action_type / outcome_status
//! / approver / cost_band / data_classification) match what
//! [`crate::filter_dims::extract_filter_dims`] re-extracts from the
//! row's `payload_json`. The columns are an indexed view of the
//! JSON; if they ever drift, `payload_json` wins and the verifier
//! flags the row. Legacy rows (pre-v67) write the columns as NULL
//! and are skipped — only non-NULL columns participate in the
//! drift check, so the verifier never trips on rows the migration
//! didn't backfill.
use crate::event::AuditKind;
use crate::filter_dims::extract_from_payload_json;
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
        "SELECT seq, kind, payload_json, parent_hash, self_hash, tenant_id, \
                worker_id, channel, action_type, outcome_status, \
                approver_id, cost_band, data_classification \
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
            "SELECT seq, kind, payload_json, parent_hash, self_hash, tenant_id, \
                    worker_id, channel, action_type, outcome_status, \
                    approver_id, cost_band, data_classification \
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
        "SELECT seq, kind, payload_json, parent_hash, self_hash, tenant_id, \
                worker_id, channel, action_type, outcome_status, \
                approver_id, cost_band, data_classification \
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
    tenant_id: String,
    // Denormalized filter columns (migration v67). NULL for legacy
    // rows; the verifier only checks drift on non-NULL columns.
    worker_id: Option<String>,
    channel: Option<String>,
    action_type: Option<String>,
    outcome_status: Option<String>,
    approver_id: Option<String>,
    cost_band: Option<String>,
    data_classification: Option<String>,
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
        // Denorm sanity belt (migration v67): if any of the
        // denormalized filter columns is non-NULL, re-extract
        // from payload_json and require an exact match. Legacy
        // rows (all columns NULL) skip the check.
        if let Some(reason) = denorm_drift_reason(&row) {
            report.ok = false;
            report.breaks.push(ChainBreak {
                seq: row.seq,
                kind: row.kind.clone(),
                reason,
            });
        }
        // Stream `mission-enterprise-cross-tenant-audit`: denorm
        // drift check. When `payload_json.tenant_id` is present it
        // must equal the `tenant_id` column. Historical rows
        // (pre-migration 63) carry `__legacy__` in the column and no
        // tenant_id key in the payload — those are intentionally
        // skipped here so verify_all keeps clean over the legacy
        // corpus.
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&row.payload_json) {
            if let Some(payload_tenant) = parsed.get("tenant_id").and_then(|v| v.as_str()) {
                if payload_tenant != row.tenant_id {
                    report.ok = false;
                    report.breaks.push(ChainBreak {
                        seq: row.seq,
                        kind: row.kind.clone(),
                        reason: format!(
                            "tenant_id denorm drift: column={} payload={payload_tenant}",
                            row.tenant_id
                        ),
                    });
                }
            }
        }
        prev_self_hash = Some(self_h);
    }
    report
}

/// Returns `Some(reason)` when a non-NULL denormalized filter
/// column drifts from what the canonical `payload_json` re-extracts
/// to; `None` when the row is legacy (all columns NULL) or when
/// every populated column matches the JSON.
fn denorm_drift_reason(row: &RawRow) -> Option<String> {
    let any_non_null = row.worker_id.is_some()
        || row.channel.is_some()
        || row.action_type.is_some()
        || row.outcome_status.is_some()
        || row.approver_id.is_some()
        || row.cost_band.is_some()
        || row.data_classification.is_some();
    if !any_non_null {
        return None;
    }
    let extracted = extract_from_payload_json(&row.payload_json)?;
    for (name, on_disk, from_json) in [
        ("worker_id", &row.worker_id, &extracted.worker_id),
        ("channel", &row.channel, &extracted.channel),
        ("action_type", &row.action_type, &extracted.action_type),
        (
            "outcome_status",
            &row.outcome_status,
            &extracted.outcome_status,
        ),
        ("approver_id", &row.approver_id, &extracted.approver_id),
        ("cost_band", &row.cost_band, &extracted.cost_band),
        (
            "data_classification",
            &row.data_classification,
            &extracted.data_classification,
        ),
    ]
    .into_iter()
    .filter(|(_, on_disk, _)| on_disk.is_some())
    {
        if on_disk != from_json {
            return Some(format!(
                "denorm column `{name}` drifted from payload_json \
                 (column={on_disk:?}, payload={from_json:?})"
            ));
        }
    }
    None
}
