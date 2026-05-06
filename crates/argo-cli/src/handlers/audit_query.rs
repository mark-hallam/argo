//! `argo audit query` H-stack filters.
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::DateTime;
use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite};

const DECISIONS: &[&str] = &["auto_approve", "ask", "block"];
const ESCALATION_KINDS: &[&str] = &["escalation_exhausted", "handoff_emitted", "handoff_settled"];

pub struct QueryArgs {
    pub decision: Option<String>,
    pub worker: Option<String>,
    pub session: Option<String>,
    pub escalation: bool,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: u32,
    pub json: bool,
}

#[derive(Debug)]
struct NormalizedQuery {
    decision: Option<String>,
    worker: Option<String>,
    session: Option<String>,
    session_id: Option<SessionId>,
    escalation: bool,
    since: Option<String>,
    since_micros: Option<i64>,
    until: Option<String>,
    until_micros: Option<i64>,
    limit: u32,
    json: bool,
}

#[derive(Serialize)]
struct QueryEnvelope<'a> {
    decision: Option<&'a str>,
    worker: Option<&'a str>,
    session: Option<&'a str>,
    escalation: bool,
    since: Option<&'a str>,
    until: Option<&'a str>,
    limit: u32,
}

#[derive(sqlx::FromRow)]
struct RawAuditRow {
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

#[derive(Serialize)]
struct AuditQueryRow {
    seq: i64,
    id: String,
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical_id: Option<String>,
    kind: String,
    payload: serde_json::Value,
    parent_hash: String,
    self_hash: String,
    created_at_micros: i64,
}

pub async fn run(store: &SessionStore, args: QueryArgs) -> ArgoResult<()> {
    let query = NormalizedQuery::new(args)?;
    let rows = query_rows(store, &query).await?;
    if query.json {
        print_json(&query, &rows)?;
    } else {
        print_text(&query, &rows);
    }
    Ok(())
}

impl NormalizedQuery {
    fn new(args: QueryArgs) -> ArgoResult<Self> {
        let decision = normalize_decision(args.decision)?;
        let session_id = match args.session.as_deref() {
            Some(raw) => Some(SessionId(uuid::Uuid::parse_str(raw).map_err(|e| {
                ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
            })?)),
            None => None,
        };
        let since_micros = parse_timestamp(args.since.as_deref(), "--since")?;
        let until_micros = parse_timestamp(args.until.as_deref(), "--until")?;
        if let (Some(since), Some(until)) = (since_micros, until_micros) {
            if since > until {
                return Err(ArgoError::Provider(
                    "invalid audit query time range: --since is after --until".into(),
                ));
            }
        }
        Ok(Self {
            decision,
            worker: args.worker,
            session: args.session,
            session_id,
            escalation: args.escalation,
            since: args.since,
            since_micros,
            until: args.until,
            until_micros,
            limit: args.limit.min(1000),
            json: args.json,
        })
    }

    fn envelope(&self) -> QueryEnvelope<'_> {
        QueryEnvelope {
            decision: self.decision.as_deref(),
            worker: self.worker.as_deref(),
            session: self.session.as_deref(),
            escalation: self.escalation,
            since: self.since.as_deref(),
            until: self.until.as_deref(),
            limit: self.limit,
        }
    }
}

async fn query_rows(
    store: &SessionStore,
    query: &NormalizedQuery,
) -> ArgoResult<Vec<AuditQueryRow>> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT a.seq, a.id, a.session_id, s.vertical_id, a.kind, \
         a.payload_json, a.parent_hash, a.self_hash, a.created_at \
         FROM audit_record a LEFT JOIN session s ON s.id = a.session_id WHERE 1=1",
    );
    if let Some(decision) = query.decision.as_deref() {
        qb.push(" AND a.kind = 'auto_approval_decision'");
        qb.push(" AND json_extract(a.payload_json, '$.decision') = ")
            .push_bind(decision);
    }
    if let Some(worker) = query.worker.as_deref() {
        qb.push(" AND (");
        qb.push("json_extract(a.payload_json, '$.worker_id') = ")
            .push_bind(worker);
        qb.push(" OR json_extract(a.payload_json, '$.from_worker_id') = ")
            .push_bind(worker);
        qb.push(" OR json_extract(a.payload_json, '$.to.worker_id') = ")
            .push_bind(worker);
        qb.push(")");
    }
    if let Some(session_id) = query.session_id {
        qb.push(" AND a.session_id = ")
            .push_bind(session_id.0.as_bytes().to_vec());
    }
    if query.escalation {
        qb.push(" AND a.kind IN (");
        let mut separated = qb.separated(", ");
        for kind in ESCALATION_KINDS {
            separated.push_bind(kind);
        }
        separated.push_unseparated(")");
    }
    if let Some(since) = query.since_micros {
        qb.push(" AND a.created_at >= ").push_bind(since);
    }
    if let Some(until) = query.until_micros {
        qb.push(" AND a.created_at <= ").push_bind(until);
    }
    qb.push(" ORDER BY a.seq DESC LIMIT ")
        .push_bind(i64::from(query.limit));

    let rows = qb
        .build_query_as::<RawAuditRow>()
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("audit query: {e}")))?;
    rows.into_iter().map(row_to_output).collect()
}

fn row_to_output(row: RawAuditRow) -> ArgoResult<AuditQueryRow> {
    let payload = serde_json::from_str(&row.payload_json)
        .map_err(|e| ArgoError::Provider(format!("audit payload parse: {e}")))?;
    Ok(AuditQueryRow {
        seq: row.seq,
        id: uuid_or_hex(&row.id),
        session_id: row.session_id.as_deref().map(uuid_or_hex),
        vertical_id: row.vertical_id,
        kind: row.kind,
        payload,
        parent_hash: hex::encode(row.parent_hash),
        self_hash: hex::encode(row.self_hash),
        created_at_micros: row.created_at,
    })
}

fn print_json(query: &NormalizedQuery, rows: &[AuditQueryRow]) -> ArgoResult<()> {
    let payload = serde_json::json!({
        "query": query.envelope(),
        "count": rows.len(),
        "rows": rows,
    });
    let serialized = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("serialize audit query: {e}")))?;
    println!("{serialized}");
    Ok(())
}

fn print_text(query: &NormalizedQuery, rows: &[AuditQueryRow]) {
    if rows.is_empty() {
        println!("(no audit rows matched query)");
        return;
    }
    println!(
        "audit query: {} row{} (limit {})",
        rows.len(),
        if rows.len() == 1 { "" } else { "s" },
        query.limit
    );
    for row in rows {
        println!(
            "  seq={:<6} kind={:<24} session={} worker={}",
            row.seq,
            row.kind,
            row.session_id.as_deref().unwrap_or("-"),
            worker_label(&row.payload),
        );
    }
}

fn worker_label(payload: &serde_json::Value) -> &str {
    payload
        .get("worker_id")
        .or_else(|| payload.get("from_worker_id"))
        .or_else(|| payload.pointer("/to/worker_id"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-")
}

fn normalize_decision(decision: Option<String>) -> ArgoResult<Option<String>> {
    match decision {
        Some(raw) if DECISIONS.contains(&raw.as_str()) => Ok(Some(raw)),
        Some(raw) => Err(ArgoError::Provider(format!(
            "invalid decision '{raw}' (expected auto_approve, ask, or block)"
        ))),
        None => Ok(None),
    }
}

fn parse_timestamp(raw: Option<&str>, flag: &str) -> ArgoResult<Option<i64>> {
    raw.map(|value| {
        DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.timestamp_micros())
            .map_err(|e| {
                ArgoError::Provider(format!(
                    "invalid {flag} timestamp '{value}': {e} \
                     (expected RFC3339/ISO-8601, e.g. 2026-05-06T13:00:00Z)"
                ))
            })
    })
    .transpose()
}

fn uuid_or_hex(bytes: &[u8]) -> String {
    uuid::Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| hex::encode(bytes))
}
