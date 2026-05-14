//! Indexed SQL branch for `routes_audit_feed::list_response`
//! (Stream `mission-enterprise-audit-full-text-search`).
//!
//! When migration v67 is applied, the J14 audit-feed handler delegates
//! its raw-row fetch to [`fetch_raw_rows`] here. The branch builds a
//! `WHERE` clause that ANDs every active dim filter as
//! `(col IN (?,…) OR col IS NULL)` plus the existing
//! `created_at >= since_micros` range bound. The `OR col IS NULL`
//! keeps legacy (pre-migration) rows flowing through so the
//! `routes_audit_feed::row_passes_filters` app-layer pass can
//! re-filter them from `payload_json` — that pass is the source of
//! correctness; if the two ever diverge, the app-layer pass is
//! right.
//!
//! Split out of `routes_audit_feed.rs` per HARD RULE 6 (500-LOC
//! per-file ceiling).

use super::routes_audit_feed::{FeedParams, RAW_FETCH_CAP};
use argo_audit::{tail_rows_filtered, ExportedRow, TailFilters};
use argo_store::SessionStore;
use std::sync::Arc;

/// The 7 filter columns added by migration v67. Static — used by
/// [`indexed_fetch`] to build the SQL `WHERE` clause and by
/// [`migration_applied`] to detect presence via `pragma_table_info`.
const FILTER_COLUMN_NAMES: &[&str] = &[
    "worker_id",
    "channel",
    "action_type",
    "outcome_status",
    "approver_id",
    "cost_band",
    "data_classification",
];

/// Migration-aware row fetcher. When v67 columns are present,
/// uses the indexed SQL path; otherwise falls back to the existing
/// `tail_rows_filtered` `created_at`-bounded fetch. Both paths
/// produce the same `ExportedRow` shape so the downstream filter +
/// paginate logic is path-agnostic — the SQL path simply pre-prunes
/// new rows so the app-layer pass has less to chew through.
///
/// # Errors
/// Returns [`argo_types::ArgoError::Provider`] on SQL failure.
pub(super) async fn fetch_raw_rows(
    store: &Arc<SessionStore>,
    params: &FeedParams,
) -> Result<Vec<ExportedRow>, argo_types::ArgoError> {
    if migration_applied(store).await {
        indexed_fetch(store, params).await
    } else {
        let filters = TailFilters {
            since_micros: params.since_micros,
            ..Default::default()
        };
        tail_rows_filtered(store, RAW_FETCH_CAP, &filters).await
    }
}

/// Cheap presence probe for the v67 filter columns via
/// `pragma_table_info`. ~10µs on warm SQLite — dwarfed by the
/// subsequent row fetch, so we don't bother memoizing per pool.
async fn migration_applied(store: &Arc<SessionStore>) -> bool {
    let names: Vec<(String,)> =
        match sqlx::query_as("SELECT name FROM pragma_table_info('audit_record')")
            .fetch_all(store.pool())
            .await
        {
            Ok(r) => r,
            Err(_) => return false,
        };
    FILTER_COLUMN_NAMES
        .iter()
        .all(|col| names.iter().any(|(n,)| n == col))
}

async fn indexed_fetch(
    store: &Arc<SessionStore>,
    params: &FeedParams,
) -> Result<Vec<ExportedRow>, argo_types::ArgoError> {
    let dim_filters: [(&str, &[String]); 7] = [
        ("worker_id", &params.worker),
        ("channel", &params.channel),
        ("action_type", &params.action_type),
        ("outcome_status", &params.status),
        ("approver_id", &params.approver),
        ("cost_band", &params.cost_band),
        ("data_classification", &params.classification),
    ];
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_strings: Vec<String> = Vec::new();
    if params.since_micros.is_some() {
        where_clauses.push("a.created_at >= ?".into());
    }
    for (col, values) in dim_filters {
        if values.is_empty() {
            continue;
        }
        let placeholders = vec!["?"; values.len()].join(",");
        // `col` is a static identifier from FILTER_COLUMN_NAMES —
        // SQL injection isn't possible; the values bind through `?`.
        where_clauses.push(format!("({col} IN ({placeholders}) OR {col} IS NULL)"));
        bind_strings.extend(values.iter().cloned());
    }
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };
    let sql = format!(
        "SELECT a.seq, a.id, a.session_id, s.vertical_id, a.kind, \
                a.payload_json, a.parent_hash, a.self_hash, a.created_at \
         FROM (\
             SELECT seq, id, session_id, kind, payload_json, \
                    parent_hash, self_hash, created_at \
             FROM audit_record a{where_sql} \
             ORDER BY seq DESC LIMIT ?\
         ) a \
         LEFT JOIN session s ON s.id = a.session_id \
         ORDER BY a.seq ASC"
    );
    let mut q = sqlx::query_as::<_, IndexedRow>(&sql);
    if let Some(s) = params.since_micros {
        q = q.bind(s);
    }
    for v in bind_strings {
        q = q.bind(v);
    }
    q = q.bind(i64::from(RAW_FETCH_CAP));
    let rows = q
        .fetch_all(store.pool())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("audit indexed fetch: {e}")))?;
    rows.into_iter().map(indexed_to_exported).collect()
}

#[derive(sqlx::FromRow)]
struct IndexedRow {
    seq: i64,
    id: Vec<u8>,
    session_id: Option<Vec<u8>>,
    vertical_id: Option<String>,
    kind: String,
    payload_json: String,
    parent_hash: Vec<u8>,
    self_hash: Vec<u8>,
    created_at: i64,
}

fn indexed_to_exported(row: IndexedRow) -> Result<ExportedRow, argo_types::ArgoError> {
    Ok(ExportedRow {
        seq: row.seq,
        id: hex::encode(&row.id),
        session_id: row.session_id.map(|b| hex::encode(&b)),
        vertical_id: row.vertical_id,
        kind: row.kind,
        payload: serde_json::from_str(&row.payload_json)
            .map_err(|e| argo_types::ArgoError::Provider(format!("audit payload parse: {e}")))?,
        parent_hash: hex::encode(&row.parent_hash),
        self_hash: hex::encode(&row.self_hash),
        created_at_micros: row.created_at,
    })
}
