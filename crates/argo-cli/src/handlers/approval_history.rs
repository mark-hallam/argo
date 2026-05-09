//! `argo approval history` read-side search.
//!
//! Maps `--since/--until/--decision/--worker/--action-kind/--limit/--format`
//! to a single parameterised SELECT against the worker-action ledger.
//! No schema mutation, no payload search.

use crate::cli::ApprovalHistoryArgs;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 1000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Param {
    Blob(Vec<u8>),
    Text(String),
    I64(i64),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ApprovalRow {
    pub id: String,
    pub requested_at: String,
    pub decided_at: Option<String>,
    pub decision: String,
    pub worker: String,
    pub action_kind: String,
    pub decided_by: Option<String>,
    pub latency_secs: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Table,
    Json,
    Markdown,
}

pub async fn run(args: ApprovalHistoryArgs) -> ArgoResult<()> {
    if args.limit > MAX_LIMIT {
        eprintln!(
            "warning: --limit {} exceeds {}; clamped",
            args.limit, MAX_LIMIT
        );
    }
    let format = OutputFormat::parse(&args.format)?;
    let (_memory, workspace, store) =
        super::worker_actions_transitions::open_workspace_memory_with_store().await?;
    let mut query_args = args;
    query_args.limit = clamped_limit(query_args.limit);
    let (sql, params) = build_query_at(&query_args, workspace, Utc::now())?;
    let rows = fetch_rows(store.pool(), &sql, params).await?;
    print!("{}", render(&rows, format));
    Ok(())
}

pub(crate) fn build_query_at(
    args: &ApprovalHistoryArgs,
    workspace: WorkspaceId,
    now: DateTime<Utc>,
) -> ArgoResult<(String, Vec<Param>)> {
    let mut sql = base_sql();
    let mut params = vec![Param::Blob(workspace.0.as_bytes().to_vec())];
    if let Some(since) = args.since.as_deref() {
        sql.push_str(" AND requested_at >= ?");
        params.push(Param::Text(super::approval_export::parse_relative_or_rfc3339(since, now)?));
    }
    if let Some(until) = args.until.as_deref() {
        sql.push_str(" AND requested_at <= ?");
        params.push(Param::Text(super::approval_export::parse_relative_or_rfc3339(until, now)?));
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
    sql.push_str(" ORDER BY requested_at DESC, id ASC LIMIT ?");
    params.push(Param::I64(i64::from(clamped_limit(args.limit))));
    Ok((sql, params))
}

pub(crate) fn render(rows: &[ApprovalRow], format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => render_ndjson(rows),
        OutputFormat::Table => render_table(rows, false),
        OutputFormat::Markdown => render_table(rows, true),
    }
}

fn base_sql() -> String {
    r#"WITH action_rows AS (
       SELECT json_extract(body,'$.action_id') id,
              json_extract(body,'$.created_at') created_at,
              COALESCE(json_extract(body,'$.payload.worker_id'), json_extract(body,'$.worker_id')) worker,
              json_extract(body,'$.action_type') action_kind,
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
       SELECT id, created_at requested_at, worker, action_kind FROM first_rows WHERE rn = 1),
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
              r.worker, r.action_kind, d.decided_by
         FROM requests r LEFT JOIN decision_rows d ON d.id = r.id AND d.rn = 1)
     SELECT id, requested_at, decided_at, decision, worker, action_kind, decided_by
       FROM joined WHERE 1=1"#
        .to_string()
}

async fn fetch_rows(
    pool: &SqlitePool,
    sql: &str,
    params: Vec<Param>,
) -> ArgoResult<Vec<ApprovalRow>> {
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
        .map_err(|e| ArgoError::Provider(format!("approval history query: {e}")))?;
    rows.into_iter().map(row_from_sql).collect()
}

fn row_from_sql(row: sqlx::sqlite::SqliteRow) -> ArgoResult<ApprovalRow> {
    let requested_at: String = row.get("requested_at");
    let decided_at: Option<String> = row.get("decided_at");
    let latency_secs = latency_secs(&requested_at, decided_at.as_deref());
    Ok(ApprovalRow {
        id: row.get("id"),
        requested_at,
        decided_at,
        decision: row.get("decision"),
        worker: row.get("worker"),
        action_kind: row.get("action_kind"),
        decided_by: row.get("decided_by"),
        latency_secs,
    })
}

fn latency_secs(requested_at: &str, decided_at: Option<&str>) -> Option<i64> {
    let requested = DateTime::parse_from_rfc3339(requested_at).ok()?;
    let decided = DateTime::parse_from_rfc3339(decided_at?).ok()?;
    Some((decided - requested).num_seconds())
}

fn clamped_limit(limit: u32) -> u32 {
    if limit == 0 {
        DEFAULT_LIMIT
    } else {
        limit.min(MAX_LIMIT)
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

impl OutputFormat {
    fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "markdown" => Ok(Self::Markdown),
            other => Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected table, json, or markdown)"
            ))),
        }
    }
}

const COLUMNS: [&str; 8] = [
    "id",
    "requested_at",
    "decided_at",
    "decision",
    "worker",
    "action_kind",
    "decided_by",
    "latency_secs",
];

fn render_ndjson(rows: &[ApprovalRow]) -> String {
    let mut out = String::new();
    for row in rows {
        match serde_json::to_string(row) {
            Ok(line) => {
                out.push_str(&line);
                out.push('\n');
            }
            Err(_) => out.push_str("{}\n"),
        }
    }
    out
}

fn render_table(rows: &[ApprovalRow], markdown: bool) -> String {
    if rows.is_empty() {
        return "(no approval history rows)\n".into();
    }
    let cells: Vec<[String; 8]> = rows.iter().map(row_cells).collect();
    if markdown {
        render_markdown_table(&cells)
    } else {
        render_text_table(&cells)
    }
}

fn row_cells(row: &ApprovalRow) -> [String; 8] {
    [
        row.id.clone(),
        row.requested_at.clone(),
        row.decided_at.clone().unwrap_or_else(|| "-".into()),
        row.decision.clone(),
        row.worker.clone(),
        row.action_kind.clone(),
        row.decided_by.clone().unwrap_or_else(|| "-".into()),
        row.latency_secs
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into()),
    ]
}

fn render_markdown_table(cells: &[[String; 8]]) -> String {
    let mut out = String::new();
    out.push_str("| ");
    out.push_str(&COLUMNS.join(" | "));
    out.push_str(" |\n| ");
    out.push_str(&vec!["---"; COLUMNS.len()].join(" | "));
    out.push_str(" |\n");
    for row in cells {
        out.push_str("| ");
        let escaped: Vec<String> = row.iter().map(|c| md_cell(c)).collect();
        out.push_str(&escaped.join(" | "));
        out.push_str(" |\n");
    }
    out
}

fn md_cell(raw: &str) -> String {
    raw.replace('|', "\\|").replace('\n', " ")
}

fn render_text_table(cells: &[[String; 8]]) -> String {
    let mut widths = [0usize; 8];
    for (idx, header) in COLUMNS.iter().enumerate() {
        widths[idx] = header.len();
    }
    for row in cells {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    push_padded(&mut out, &COLUMNS.map(|s| s.to_string()), &widths);
    let separators: [String; 8] = std::array::from_fn(|idx| "-".repeat(widths[idx]));
    push_padded(&mut out, &separators, &widths);
    for row in cells {
        push_padded(&mut out, row, &widths);
    }
    out
}

fn push_padded(out: &mut String, row: &[String; 8], widths: &[usize; 8]) {
    for (idx, cell) in row.iter().enumerate() {
        let pad = widths[idx].saturating_sub(cell.chars().count());
        out.push_str(cell);
        for _ in 0..pad {
            out.push(' ');
        }
        if idx + 1 < row.len() {
            out.push_str("  ");
        }
    }
    out.push('\n');
}

#[cfg(test)]
#[path = "approval_history_tests.rs"]
mod tests;
