//! `argo approval export` audit log exporter.

use crate::cli::ApprovalExportArgs;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::{io::Write, path::Path};

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 1000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Param {
    Blob(Vec<u8>),
    Text(String),
    I64(i64),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ApprovalExportRow {
    pub id: String,
    pub requested_at: String,
    pub decided_at: Option<String>,
    pub decision: String,
    pub worker: String,
    pub action_kind: String,
    pub decided_by: Option<String>,
    pub latency_secs: Option<i64>,
    pub payload_excerpt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportFormat {
    Csv,
    Json,
    Jsonl,
}

pub async fn run(args: ApprovalExportArgs) -> ArgoResult<()> {
    let format = ExportFormat::parse(&args.format)?;
    let limit = clamped_limit(args.limit);
    if args.limit > MAX_LIMIT {
        eprintln!("warning: --limit {} exceeds {}; clamped", args.limit, MAX_LIMIT);
    }
    ensure_writable(&args.out, args.force)?;
    let (_memory, workspace, store) = super::worker_actions_transitions::open_workspace_memory_with_store().await?;
    let mut query_args = args.clone();
    query_args.limit = limit;
    let (sql, params) = build_query_at(&query_args, workspace, Utc::now())?;
    let rows = fetch_rows(store.pool(), &sql, params).await?;
    let body = render(&rows, format)?;
    let sha256 = hex::encode(Sha256::digest(&body));
    write_body(&args.out, &body)?;
    eprintln!("wrote {} rows, sha256={sha256}", rows.len());
    Ok(())
}

pub(crate) fn build_query_at(
    args: &ApprovalExportArgs,
    workspace: WorkspaceId,
    now: DateTime<Utc>,
) -> ArgoResult<(String, Vec<Param>)> {
    // TODO: dedupe with `approval_history` once that handler is on main.
    let mut sql = base_sql();
    let mut params = vec![Param::Blob(workspace.0.as_bytes().to_vec())];
    if let Some(since) = args.since.as_deref() {
        sql.push_str(" AND requested_at >= ?");
        params.push(Param::Text(parse_relative_or_rfc3339(since, now)?));
    }
    if let Some(until) = args.until.as_deref() {
        sql.push_str(" AND requested_at <= ?");
        params.push(Param::Text(parse_relative_or_rfc3339(until, now)?));
    }
    if !args.decision.is_empty() {
        sql.push_str(" AND decision IN (");
        for (idx, decision) in args.decision.iter().enumerate() {
            if idx > 0 {
                sql.push(',');
            }
            sql.push('?');
            params.push(Param::Text(parse_decision(decision)?.to_string()));
        }
        sql.push(')');
    }
    if let Some(worker) = args.worker.as_deref() {
        sql.push_str(" AND worker = ?");
        params.push(Param::Text(worker.to_string()));
    }
    if let Some(kind) = args.action_kind.as_deref() {
        sql.push_str(" AND action_kind = ?");
        params.push(Param::Text(kind.to_string()));
    }
    sql.push_str(" ORDER BY requested_at ASC, id ASC LIMIT ?");
    params.push(Param::I64(i64::from(clamped_limit(args.limit))));
    Ok((sql, params))
}

pub(crate) fn render(rows: &[ApprovalExportRow], format: ExportFormat) -> ArgoResult<Vec<u8>> {
    match format {
        ExportFormat::Csv => render_csv(rows),
        ExportFormat::Json => serde_json::to_vec(rows).map_err(|e| ArgoError::Provider(format!("approval export JSON: {e}"))),
        ExportFormat::Jsonl => render_jsonl(rows),
    }
}

fn base_sql() -> String {
    r#"WITH action_rows AS (
       SELECT json_extract(body,'$.action_id') id,
              json_extract(body,'$.created_at') created_at,
              COALESCE(json_extract(body,'$.payload.worker_id'), json_extract(body,'$.worker_id')) worker,
              json_extract(body,'$.action_type') action_kind,
              json_extract(body,'$.payload') payload,
              json_extract(body,'$.approved_by') approved_by,
              json_extract(body,'$.approved_at') approved_at,
              lower(COALESCE(json_extract(body,'$.result.decision'),'')) result_decision
         FROM memory_entry
        WHERE workspace_id = ? AND file = 'MEMORY.md' AND superseded_by IS NULL
          AND json_valid(body) AND json_extract(body,'$.schema_key') = 'worker_action'
          AND json_extract(body,'$.requires_approval') = 1),
     first_rows AS (
       SELECT *, ROW_NUMBER() OVER (PARTITION BY id ORDER BY created_at ASC) rn FROM action_rows),
     requests AS (
       SELECT id, created_at requested_at, worker, action_kind, payload FROM first_rows WHERE rn = 1),
     decision_rows AS (
       SELECT id, COALESCE(approved_at, created_at) decided_at, approved_by decided_by,
              CASE WHEN result_decision IN ('deny','reject') THEN 'deny'
                   WHEN result_decision = 'timeout' THEN 'timeout'
                   WHEN approved_at IS NOT NULL THEN 'approve' END decision,
              ROW_NUMBER() OVER (PARTITION BY id ORDER BY created_at DESC) rn
         FROM action_rows
        WHERE result_decision IN ('deny','reject','timeout') OR approved_at IS NOT NULL),
     joined AS (
       SELECT r.id, r.requested_at, d.decided_at, COALESCE(d.decision,'pending') decision,
              r.worker, r.action_kind, d.decided_by, r.payload
         FROM requests r LEFT JOIN decision_rows d ON d.id = r.id AND d.rn = 1)
     SELECT id, requested_at, decided_at, decision, worker, action_kind, decided_by, payload
       FROM joined WHERE 1=1"#
        .to_string()
}

async fn fetch_rows(
    pool: &SqlitePool,
    sql: &str,
    params: Vec<Param>,
) -> ArgoResult<Vec<ApprovalExportRow>> {
    let mut q = sqlx::query(sql);
    for param in params {
        q = match param {
            Param::Blob(v) => q.bind(v),
            Param::Text(v) => q.bind(v),
            Param::I64(v) => q.bind(v),
        };
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("approval export query: {e}")))?;
    rows.into_iter().map(row_from_sql).collect()
}

fn row_from_sql(row: sqlx::sqlite::SqliteRow) -> ArgoResult<ApprovalExportRow> {
    let requested_at: String = row.get("requested_at");
    let decided_at: Option<String> = row.get("decided_at");
    Ok(ApprovalExportRow {
        id: row.get("id"),
        requested_at: requested_at.clone(),
        decided_at: decided_at.clone(),
        decision: row.get("decision"),
        worker: row.get("worker"),
        action_kind: row.get("action_kind"),
        decided_by: row.get("decided_by"),
        latency_secs: latency_secs(&requested_at, decided_at.as_deref()),
        payload_excerpt: payload_excerpt(row.get::<Option<String>, _>("payload").as_deref()),
    })
}

fn render_csv(rows: &[ApprovalExportRow]) -> ArgoResult<Vec<u8>> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer
        .write_record([
            "id",
            "requested_at",
            "decided_at",
            "decision",
            "worker",
            "action_kind",
            "decided_by",
            "latency_secs",
            "payload_excerpt",
        ])
        .map_err(|e| ArgoError::Provider(format!("approval export CSV header: {e}")))?;
    for row in rows {
        writer
            .serialize(row)
            .map_err(|e| ArgoError::Provider(format!("approval export CSV row: {e}")))?;
    }
    writer
        .into_inner()
        .map_err(|e| ArgoError::Provider(format!("approval export CSV flush: {e}")))
}

fn render_jsonl(rows: &[ApprovalExportRow]) -> ArgoResult<Vec<u8>> {
    let mut out = Vec::new();
    for row in rows {
        serde_json::to_writer(&mut out, row)
            .map_err(|e| ArgoError::Provider(format!("approval export JSONL: {e}")))?;
        out.push(b'\n');
    }
    Ok(out)
}

fn ensure_writable(out: &str, force: bool) -> ArgoResult<()> {
    if out != "-" && Path::new(out).exists() && !force {
        return Err(ArgoError::Provider(format!(
            "refusing to overwrite existing file: {out} (use --force)"
        )));
    }
    Ok(())
}

fn write_body(out: &str, body: &[u8]) -> ArgoResult<()> {
    if out == "-" {
        std::io::stdout()
            .lock()
            .write_all(body)
            .map_err(|e| ArgoError::Provider(format!("approval export stdout: {e}")))
    } else {
        std::fs::write(out, body)
            .map_err(|e| ArgoError::Provider(format!("approval export write {out}: {e}")))
    }
}

pub(crate) fn parse_relative_or_rfc3339(raw: &str, now: DateTime<Utc>) -> ArgoResult<String> {
    let trimmed = raw.trim();
    if let Some(duration) = parse_relative(trimmed)? {
        return Ok((now - duration).to_rfc3339());
    }
    DateTime::parse_from_rfc3339(trimmed)
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
        .map_err(|e| ArgoError::Schema(format!("invalid timestamp '{raw}': {e}")))
}

fn parse_relative(raw: &str) -> ArgoResult<Option<Duration>> {
    if raw.len() < 2 {
        return Ok(None);
    }
    let (digits, unit) = raw.split_at(raw.len() - 1);
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Ok(None);
    }
    let value = digits
        .parse::<i64>()
        .map_err(|e| ArgoError::Schema(format!("invalid duration '{raw}': {e}")))?;
    if value <= 0 {
        return Err(ArgoError::Provider(format!(
            "duration must be positive: {raw}"
        )));
    }
    match unit {
        "m" => Ok(Some(Duration::minutes(value))),
        "h" => Ok(Some(Duration::hours(value))),
        "d" => Ok(Some(Duration::days(value))),
        _ => Ok(None),
    }
}

fn parse_decision(raw: &str) -> ArgoResult<&str> {
    match raw {
        "approve" | "deny" | "pending" | "timeout" => Ok(raw),
        other => Err(ArgoError::Provider(format!(
            "unknown decision '{other}' (expected approve, deny, pending, or timeout)"
        ))),
    }
}

fn clamped_limit(limit: u32) -> u32 {
    if limit == 0 {
        DEFAULT_LIMIT
    } else {
        limit.min(MAX_LIMIT)
    }
}

fn latency_secs(requested_at: &str, decided_at: Option<&str>) -> Option<i64> {
    let requested = DateTime::parse_from_rfc3339(requested_at).ok()?;
    let decided = DateTime::parse_from_rfc3339(decided_at?).ok()?;
    Some((decided - requested).num_seconds())
}

pub(crate) fn payload_excerpt(raw: Option<&str>) -> String {
    let normalized = raw.unwrap_or_default().replace('\n', "\\n");
    let mut chars = normalized.chars();
    let excerpt: String = chars.by_ref().take(80).collect();
    if chars.next().is_some() {
        format!("{excerpt}...")
    } else {
        excerpt
    }
}

impl ExportFormat {
    fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            "jsonl" => Ok(Self::Jsonl),
            other => Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected csv, json, or jsonl)"
            ))),
        }
    }
}

#[cfg(test)]
#[path = "approval_export_tests.rs"]
mod tests;
