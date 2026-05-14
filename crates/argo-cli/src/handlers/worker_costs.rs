//! Read-only worker cost attribution over the `worker_budget`
//! ledger.

use crate::cli::cli_worker_costs_cmds::{WorkerCostsBreakdown, WorkerCostsCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite};
use std::collections::BTreeMap;
use uuid::Uuid;

const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/14__worker_budget.sql");

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkerCostsReport {
    pub worker_id: String,
    pub tenant_id: Option<String>,
    pub since: Option<String>,
    pub total_usd_micros: u64,
    pub total_usd: f64,
    pub row_count: usize,
    pub breakdown: Option<String>,
    pub breakdown_rows: Vec<BreakdownRow>,
    pub recent_charges: Vec<WorkerChargeRow>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkerChargeRow {
    pub tenant_id_hex: String,
    pub worker_id: String,
    pub action_id: String,
    pub usd_micros: u64,
    pub usd: f64,
    pub charged_at: String,
    pub charged_at_epoch: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct BreakdownRow {
    pub key: String,
    pub usd_micros: u64,
    pub row_count: usize,
}

struct TenantFilter {
    raw: String,
    bytes: Vec<u8>,
}

pub async fn run(cmd: WorkerCostsCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    let report = report(
        &store,
        &cmd.worker_id,
        cmd.tenant.as_deref(),
        cmd.since.as_deref(),
        cmd.breakdown,
        cmd.limit,
    )
    .await?;
    if cmd.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| ArgoError::Provider(format!("serialize worker costs: {e}")))?
        );
    } else {
        print!("{}", render_text(&report));
    }
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn report(
    store: &SessionStore,
    worker_id: &str,
    tenant: Option<&str>,
    since: Option<&str>,
    breakdown: Option<WorkerCostsBreakdown>,
    limit: u32,
) -> ArgoResult<WorkerCostsReport> {
    let tenant = parse_tenant(tenant)?;
    let since = parse_since(since)?;
    let rows = query_rows(store, worker_id, tenant.as_ref(), since).await?;
    let total_usd_micros = rows.iter().map(|r| r.usd_micros).sum();
    let breakdown_rows = build_breakdown(&rows, breakdown);
    let recent_charges = rows
        .iter()
        .take(limit.clamp(1, 500) as usize)
        .cloned()
        .collect();
    Ok(WorkerCostsReport {
        worker_id: worker_id.to_string(),
        tenant_id: tenant.as_ref().map(|t| t.raw.clone()),
        since: since.map(|dt| dt.to_rfc3339()),
        total_usd_micros,
        total_usd: micros_to_usd(total_usd_micros),
        row_count: rows.len(),
        breakdown: breakdown.map(|b| match b {
            WorkerCostsBreakdown::Day => "day".to_string(),
            WorkerCostsBreakdown::Action => "action".to_string(),
        }),
        breakdown_rows,
        recent_charges,
    })
}

async fn query_rows(
    store: &SessionStore,
    worker_id: &str,
    tenant: Option<&TenantFilter>,
    since: Option<DateTime<Utc>>,
) -> ArgoResult<Vec<WorkerChargeRow>> {
    ensure_budget_schema(store).await?;
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT tenant_id, worker_id, action_id, usd_micros, charged_at \
         FROM worker_budget WHERE worker_id = ",
    );
    qb.push_bind(worker_id);
    if let Some(tenant) = tenant {
        qb.push(" AND tenant_id = ").push_bind(tenant.bytes.as_slice());
    }
    if let Some(since) = since {
        qb.push(" AND charged_at >= ").push_bind(since.timestamp());
    }
    qb.push(" ORDER BY charged_at DESC, action_id DESC");
    let rows: Vec<(Vec<u8>, String, String, i64, i64)> = qb
        .build_query_as()
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("worker costs query: {e}")))?;
    Ok(rows
        .into_iter()
        .map(
            |(tenant_id, worker_id, action_id, usd_micros, charged_at)| {
                let micros = u64::try_from(usd_micros).unwrap_or(0);
                WorkerChargeRow {
                    tenant_id_hex: hex::encode(tenant_id),
                    worker_id,
                    action_id,
                    usd_micros: micros,
                    usd: micros_to_usd(micros),
                    charged_at: epoch_to_rfc3339(charged_at),
                    charged_at_epoch: charged_at,
                }
            },
        )
        .collect())
}

fn build_breakdown(
    rows: &[WorkerChargeRow],
    breakdown: Option<WorkerCostsBreakdown>,
) -> Vec<BreakdownRow> {
    let Some(breakdown) = breakdown else {
        return Vec::new();
    };
    let mut grouped: BTreeMap<String, (u64, usize)> = BTreeMap::new();
    for row in rows {
        let key = match breakdown {
            WorkerCostsBreakdown::Day => epoch_day(row.charged_at_epoch),
            WorkerCostsBreakdown::Action => row.action_id.clone(),
        };
        let entry = grouped.entry(key).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(row.usd_micros);
        entry.1 += 1;
    }
    grouped
        .into_iter()
        .map(|(key, (usd_micros, row_count))| BreakdownRow {
            key,
            usd_micros,
            row_count,
        })
        .collect()
}

async fn ensure_budget_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in WORKER_BUDGET_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker budget schema: {e}")))?;
    }
    Ok(())
}

fn parse_tenant(raw: Option<&str>) -> ArgoResult<Option<TenantFilter>> {
    raw.map(|tenant| {
        Uuid::parse_str(tenant)
            .map(|uuid| TenantFilter {
                raw: tenant.to_string(),
                bytes: uuid.as_bytes().to_vec(),
            })
            .map_err(|e| ArgoError::Schema(format!("invalid --tenant '{tenant}': {e}")))
    })
    .transpose()
}

fn parse_since(raw: Option<&str>) -> ArgoResult<Option<DateTime<Utc>>> {
    raw.map(|since| {
        DateTime::parse_from_rfc3339(since)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                ArgoError::Provider(format!(
                    "invalid --since '{since}' (expected RFC3339): {e}"
                ))
            })
    })
    .transpose()
}

fn render_text(report: &WorkerCostsReport) -> String {
    let mut out = format!("worker costs {}\n", report.worker_id);
    if let Some(tenant) = report.tenant_id.as_deref() {
        out.push_str(&format!("  tenant: {tenant}\n"));
    }
    if let Some(since) = report.since.as_deref() {
        out.push_str(&format!("  since:  {since}\n"));
    }
    out.push_str(&format!(
        "  total:  ${} ({} micros)\n  rows:   {}\n",
        format_usd(report.total_usd_micros),
        report.total_usd_micros,
        report.row_count
    ));
    if let Some(kind) = report.breakdown.as_deref() {
        out.push_str(&format!("breakdown by {kind}:\n"));
        if report.breakdown_rows.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for row in &report.breakdown_rows {
                out.push_str(&format!(
                    "  {}  ${:>10}  rows={}\n",
                    row.key,
                    format_usd(row.usd_micros),
                    row.row_count
                ));
            }
        }
    }
    out.push_str("recent charges:\n");
    if report.recent_charges.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for row in &report.recent_charges {
            out.push_str(&format!(
                "  {}  ${:>10}  action={} tenant={}\n",
                row.charged_at,
                format_usd(row.usd_micros),
                row.action_id,
                row.tenant_id_hex
            ));
        }
    }
    out
}

fn micros_to_usd(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}

fn format_usd(micros: u64) -> String {
    let usd = micros_to_usd(micros);
    if micros > 0 && usd < 0.01 {
        format!("{usd:.4}")
    } else {
        format!("{usd:.2}")
    }
}

fn epoch_to_rfc3339(epoch: i64) -> String {
    Utc.timestamp_opt(epoch, 0)
        .single()
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

fn epoch_day(epoch: i64) -> String {
    Utc.timestamp_opt(epoch, 0)
        .single()
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .format("%Y-%m-%d")
        .to_string()
}

#[cfg(test)]
#[path = "worker_costs_tests.rs"]
mod tests;
