//! `argo activity since` — recent worker activity feed.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite};
use uuid::Uuid;

use super::activity_since_render::{format_rfc3339, short_id, uuid_or_hex};

pub(crate) use super::activity_since_render::{format_text_row, print_ndjson, print_text_rows};

const MAX_LIMIT: u32 = 1000;

#[derive(Debug, Clone)]
pub(crate) struct ActivityQuery {
    pub tenant: Option<TenantFilter>,
    pub worker: Option<String>,
    pub since_micros: Option<i64>,
    pub after_seq: Option<i64>,
    pub limit: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct TenantFilter {
    raw: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActivityRow {
    pub seq: i64,
    pub id: String,
    pub session_id: Option<String>,
    pub tenant_id: Option<String>,
    pub vertical_id: Option<String>,
    pub audit_kind: String,
    pub action_kind: String,
    pub decision: Option<String>,
    pub worker_id: Option<String>,
    pub worker_short_id: String,
    pub message: String,
    pub created_at: String,
    pub created_at_micros: i64,
    pub payload: serde_json::Value,
}

#[derive(Debug, sqlx::FromRow)]
struct RawActivityRow {
    seq: i64,
    id: Vec<u8>,
    session_id: Option<Vec<u8>>,
    tenant_id: Option<Vec<u8>>,
    vertical_id: Option<String>,
    kind: String,
    payload_json: String,
    created_at: i64,
}

pub async fn run(
    duration: &str,
    limit: u32,
    tenant: Option<String>,
    worker: Option<String>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let query = ActivityQuery::recent(duration, limit, tenant, worker)?;
    let rows = query_activity(&store, &query).await?;
    if json {
        print_ndjson(&rows)
    } else {
        print_text_rows(&rows);
        Ok(())
    }
}

pub(crate) async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

impl ActivityQuery {
    pub(crate) fn recent(
        duration: &str,
        limit: u32,
        tenant: Option<String>,
        worker: Option<String>,
    ) -> ArgoResult<Self> {
        let lookback = parse_duration_micros(duration)?;
        Ok(Self {
            tenant: parse_tenant_filter(tenant)?,
            worker: normalize_worker(worker)?,
            since_micros: Some(now_micros().saturating_sub(lookback)),
            after_seq: None,
            limit: limit.min(MAX_LIMIT),
        })
    }
}

pub(crate) fn tail_query(
    since: Option<&str>,
    tenant: Option<String>,
    worker: Option<String>,
    limit: u32,
) -> ArgoResult<ActivityQuery> {
    let since_micros = since
        .map(parse_duration_micros)
        .transpose()?
        .map(|lookback| now_micros().saturating_sub(lookback));
    Ok(ActivityQuery {
        tenant: parse_tenant_filter(tenant)?,
        worker: normalize_worker(worker)?,
        since_micros,
        after_seq: None,
        limit: limit.min(MAX_LIMIT),
    })
}

pub(crate) fn after_query(base: &ActivityQuery, after_seq: i64, limit: u32) -> ActivityQuery {
    let mut next = base.clone();
    next.after_seq = Some(after_seq);
    next.since_micros = None;
    next.limit = limit.min(MAX_LIMIT);
    next
}

pub(crate) async fn query_activity(
    store: &SessionStore,
    query: &ActivityQuery,
) -> ArgoResult<Vec<ActivityRow>> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT * FROM (SELECT a.seq AS seq, a.id AS id, \
         a.session_id AS session_id, s.tenant_id AS tenant_id, \
         s.vertical_id AS vertical_id, a.kind AS kind, \
         a.payload_json AS payload_json, a.created_at AS created_at \
         FROM audit_record a LEFT JOIN session s ON s.id = a.session_id \
         WHERE 1=1",
    );
    if let Some(since) = query.since_micros {
        qb.push(" AND a.created_at >= ").push_bind(since);
    }
    if let Some(after) = query.after_seq {
        qb.push(" AND a.seq > ").push_bind(after);
    }
    if let Some(tenant) = &query.tenant {
        qb.push(" AND (s.tenant_id = ")
            .push_bind(tenant.bytes.clone())
            .push(" OR json_extract(a.payload_json, '$.tenant_id') = ")
            .push_bind(tenant.raw.clone())
            .push(")");
    }
    if let Some(worker) = &query.worker {
        qb.push(" AND (json_extract(a.payload_json, '$.worker_id') = ")
            .push_bind(worker.clone())
            .push(" OR json_extract(a.payload_json, '$.from_worker_id') = ")
            .push_bind(worker.clone())
            .push(" OR json_extract(a.payload_json, '$.to.worker_id') = ")
            .push_bind(worker.clone())
            .push(")");
    }
    qb.push(" ORDER BY a.seq DESC LIMIT ")
        .push_bind(i64::from(query.limit))
        .push(") ORDER BY seq ASC");
    let rows = qb
        .build_query_as::<RawActivityRow>()
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("activity query: {e}")))?;
    rows.into_iter().map(row_to_activity).collect()
}

pub(crate) async fn max_seq(store: &SessionStore) -> ArgoResult<i64> {
    sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("activity max seq: {e}")))
}

fn row_to_activity(row: RawActivityRow) -> ArgoResult<ActivityRow> {
    let payload: serde_json::Value = serde_json::from_str(&row.payload_json)
        .map_err(|e| ArgoError::Provider(format!("activity payload parse: {e}")))?;
    let worker_id = worker_id(&payload);
    let action_kind = action_kind(&row.kind, &payload);
    let decision = string_field(&payload, "decision").or_else(|| string_field(&payload, "status"));
    let message = message(&row.kind, &payload);
    Ok(ActivityRow {
        seq: row.seq,
        id: uuid_or_hex(&row.id),
        session_id: row.session_id.as_deref().map(uuid_or_hex),
        tenant_id: row
            .tenant_id
            .as_deref()
            .map(uuid_or_hex)
            .or_else(|| string_field(&payload, "tenant_id")),
        vertical_id: row.vertical_id,
        audit_kind: row.kind,
        action_kind,
        decision,
        worker_short_id: worker_id
            .as_deref()
            .map(short_id)
            .unwrap_or_else(|| "-".into()),
        worker_id,
        message,
        created_at: format_rfc3339(row.created_at),
        created_at_micros: row.created_at,
        payload,
    })
}

fn parse_tenant_filter(raw: Option<String>) -> ArgoResult<Option<TenantFilter>> {
    raw.map(|tenant| {
        let trimmed = tenant.trim();
        let id = Uuid::parse_str(trimmed)
            .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{trimmed}': {e}")))?;
        Ok(TenantFilter {
            raw: trimmed.to_string(),
            bytes: id.as_bytes().to_vec(),
        })
    })
    .transpose()
}

fn normalize_worker(raw: Option<String>) -> ArgoResult<Option<String>> {
    raw.map(|worker| {
        let trimmed = worker.trim();
        if trimmed.is_empty() {
            Err(ArgoError::Provider(
                "activity worker filter is empty".into(),
            ))
        } else {
            Ok(trimmed.to_string())
        }
    })
    .transpose()
}

pub(crate) fn parse_duration_micros(raw: &str) -> ArgoResult<i64> {
    let trimmed = raw.trim();
    let (digits, suffix) = trimmed.split_at(trimmed.len().saturating_sub(1));
    let value: u64 = digits.parse().map_err(|_| invalid_duration(trimmed))?;
    let seconds = match suffix {
        "s" => value,
        "m" => value
            .checked_mul(60)
            .ok_or_else(|| invalid_duration(trimmed))?,
        "h" => value
            .checked_mul(3_600)
            .ok_or_else(|| invalid_duration(trimmed))?,
        "d" => value
            .checked_mul(86_400)
            .ok_or_else(|| invalid_duration(trimmed))?,
        _ => return Err(invalid_duration(trimmed)),
    };
    let micros = seconds
        .checked_mul(1_000_000)
        .and_then(|n| i64::try_from(n).ok())
        .ok_or_else(|| invalid_duration(trimmed))?;
    Ok(micros)
}

fn invalid_duration(raw: &str) -> ArgoError {
    ArgoError::Provider(format!(
        "invalid duration '{raw}' (expected 30s, 5m, 1h, or 1d)"
    ))
}

fn now_micros() -> i64 {
    chrono::Utc::now().timestamp_micros()
}

fn worker_id(payload: &serde_json::Value) -> Option<String> {
    string_field(payload, "worker_id")
        .or_else(|| string_field(payload, "from_worker_id"))
        .or_else(|| payload.pointer("/to/worker_id").and_then(str_value))
}

fn action_kind(kind: &str, payload: &serde_json::Value) -> String {
    string_field(payload, "action_kind")
        .or_else(|| payload.pointer("/action/kind").and_then(str_value))
        .unwrap_or_else(|| kind.to_string())
}

fn message(kind: &str, payload: &serde_json::Value) -> String {
    for key in ["message", "summary", "reason", "error", "tool", "action_id"] {
        if let Some(value) = string_field(payload, key) {
            return value;
        }
    }
    if payload.is_null() || payload.as_object().is_some_and(serde_json::Map::is_empty) {
        kind.to_string()
    } else {
        compact_payload(payload)
    }
}

fn string_field(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload.get(key).and_then(str_value)
}

fn str_value(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

fn compact_payload(payload: &serde_json::Value) -> String {
    let mut out = serde_json::to_string(payload).unwrap_or_else(|_| "{}".into());
    if out.len() > 160 {
        out.truncate(157);
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_duration_suffixes() {
        assert_eq!(parse_duration_micros("30s").unwrap(), 30_000_000);
        assert_eq!(parse_duration_micros("5m").unwrap(), 300_000_000);
        assert_eq!(parse_duration_micros("1h").unwrap(), 3_600_000_000);
        assert_eq!(parse_duration_micros("1d").unwrap(), 86_400_000_000);
    }

    #[test]
    fn rejects_bad_duration_shape() {
        let err = parse_duration_micros("five minutes")
            .unwrap_err()
            .to_string();
        assert!(err.contains("expected 30s, 5m, 1h, or 1d"));
    }
}
