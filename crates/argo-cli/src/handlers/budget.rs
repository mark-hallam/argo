//! `argo budget` — per-worker budget ledger reads.
use crate::cli::{BudgetAlertsCmd, BudgetCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{worker_budget, AutoApprovalPolicy};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite};

const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/14__worker_budget.sql");

pub async fn run(cmd: BudgetCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    match cmd {
        BudgetCmd::Show { worker_id, json } => {
            let summary = budget_summary(&store, &worker_id, Utc::now()).await?;
            if json {
                println!("{}", serde_json::to_string(&summary).unwrap_or_default());
            } else {
                print!("{}", render_summary_text(&summary));
            }
        }
        BudgetCmd::Recent {
            limit,
            worker,
            since,
            json,
        } => {
            let since = parse_since(since.as_deref())?;
            let rows = recent_charges(&store, limit, worker.as_deref(), since).await?;
            if json {
                println!("{}", serde_json::to_string(&rows).unwrap_or_default());
            } else {
                print!("{}", render_recent_text(&rows, worker.as_deref(), since));
            }
        }
        BudgetCmd::Anomalies {
            worker,
            since,
            limit,
            json,
        } => {
            let since = parse_since(since.as_deref())?;
            super::budget_anomalies::list(&store, worker.as_deref(), since, limit, json).await?;
        }
        BudgetCmd::AnomalyTick { json } => {
            super::budget_anomalies::tick(&store, Utc::now(), json).await?;
        }
        BudgetCmd::Watch { worker, once, json } => {
            super::budget_watch::run(worker.as_deref(), once, json).await?;
        }
        BudgetCmd::Alerts { cmd } => match cmd {
            BudgetAlertsCmd::List {
                worker,
                since,
                limit,
                json,
            } => {
                super::budget_alerts_list::run(worker.as_deref(), since.as_deref(), limit, json)
                    .await?;
            }
            BudgetAlertsCmd::Test {
                worker,
                threshold,
                json,
            } => {
                super::budget_alerts_test::run(&worker, threshold, json).await?;
            }
        },
    }
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

#[derive(Debug, Clone, Serialize)]
struct BudgetSummary {
    worker_id: String,
    today_usd_micros: u64,
    this_month_usd_micros: u64,
    today_usd: f64,
    this_month_usd: f64,
    daily_cap_usd_micros: Option<u64>,
    monthly_cap_usd_micros: Option<u64>,
    remaining_today_usd: Option<f64>,
    remaining_month_usd: Option<f64>,
    last_charges: Vec<ChargeRow>,
}

#[derive(Debug, Clone, Serialize)]
struct ChargeRow {
    worker_id: String,
    action_id: String,
    usd_micros: u64,
    usd: f64,
    charged_at: String,
    charged_at_epoch: i64,
}

async fn budget_summary(
    store: &SessionStore,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<BudgetSummary> {
    let pool = store.pool();
    let today = worker_budget::spend_today_for_worker(pool, worker_id, now).await?;
    let month = spend_this_month_for_worker(store, worker_id, now).await?;
    let policy = worker_policy(store, worker_id).await?;
    let daily_cap = policy.as_ref().and_then(|p| p.daily_budget_usd_micros);
    let last_charges = recent_charges(store, 5, Some(worker_id), None).await?;
    Ok(BudgetSummary {
        worker_id: worker_id.to_string(),
        today_usd_micros: today,
        this_month_usd_micros: month,
        today_usd: micros_to_usd(today),
        this_month_usd: micros_to_usd(month),
        daily_cap_usd_micros: daily_cap,
        monthly_cap_usd_micros: None,
        remaining_today_usd: daily_cap.map(|cap| micros_to_usd(cap.saturating_sub(today))),
        remaining_month_usd: None,
        last_charges,
    })
}

async fn spend_this_month_for_worker(
    store: &SessionStore,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<u64> {
    ensure_budget_schema(store).await?;
    let start = month_start_utc(now).timestamp();
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(start)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("worker budget spend this month: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

async fn worker_policy(
    store: &SessionStore,
    worker_id: &str,
) -> ArgoResult<Option<AutoApprovalPolicy>> {
    if !table_exists(store, "worker").await? {
        return Ok(None);
    }
    let row: Option<(String,)> =
        sqlx::query_as("SELECT auto_approval_json FROM worker WHERE id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker policy lookup: {e}")))?;
    row.map(|(body,)| {
        serde_json::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("worker policy JSON: {e}")))
    })
    .transpose()
}

async fn recent_charges(
    store: &SessionStore,
    limit: u32,
    worker: Option<&str>,
    since: Option<DateTime<Utc>>,
) -> ArgoResult<Vec<ChargeRow>> {
    ensure_budget_schema(store).await?;
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT worker_id, action_id, usd_micros, charged_at FROM worker_budget WHERE 1=1",
    );
    if let Some(worker) = worker {
        qb.push(" AND worker_id = ").push_bind(worker);
    }
    if let Some(since) = since {
        qb.push(" AND charged_at >= ").push_bind(since.timestamp());
    }
    qb.push(" ORDER BY charged_at DESC LIMIT ")
        .push_bind(i64::from(limit.clamp(1, 500)));
    let rows: Vec<(String, String, i64, i64)> =
        qb.build_query_as()
            .fetch_all(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker budget recent: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(worker_id, action_id, usd_micros, charged_at)| {
            let micros = u64::try_from(usd_micros).unwrap_or(0);
            ChargeRow {
                worker_id,
                action_id,
                usd_micros: micros,
                usd: micros_to_usd(micros),
                charged_at: Utc
                    .timestamp_opt(charged_at, 0)
                    .single()
                    .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
                    .to_rfc3339(),
                charged_at_epoch: charged_at,
            }
        })
        .collect())
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

async fn table_exists(store: &SessionStore, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("table lookup {table}: {e}")))?;
    Ok(count > 0)
}

fn parse_since(raw: Option<&str>) -> ArgoResult<Option<DateTime<Utc>>> {
    raw.map(|s| {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                ArgoError::Provider(format!(
                    "invalid --since '{s}' (expected ISO-8601 like 2026-05-01T00:00:00Z): {e}"
                ))
            })
    })
    .transpose()
}

fn month_start_utc(now: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
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

fn render_summary_text(summary: &BudgetSummary) -> String {
    let mut out = format!("budget show worker={}\n", summary.worker_id);
    out.push_str(&format!(
        "  today:      ${:>10}\n",
        format_usd(summary.today_usd_micros)
    ));
    out.push_str(&format!(
        "  this month: ${:>10}\n",
        format_usd(summary.this_month_usd_micros)
    ));
    match summary.daily_cap_usd_micros {
        Some(cap) => out.push_str(&format!(
            "  remaining today: ${:>10}\n",
            format_usd(cap.saturating_sub(summary.today_usd_micros))
        )),
        None => out.push_str("  remaining today: (unlimited)\n"),
    }
    out.push_str("  remaining month: (unlimited)\n");
    out.push_str("last charges:\n");
    if summary.last_charges.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for row in &summary.last_charges {
            out.push_str(&format!(
                "  {}  ${:>10}  action={}\n",
                row.charged_at,
                format_usd(row.usd_micros),
                row.action_id
            ));
        }
    }
    out
}

fn render_recent_text(
    rows: &[ChargeRow],
    worker: Option<&str>,
    since: Option<DateTime<Utc>>,
) -> String {
    if rows.is_empty() {
        return "(no worker budget charges)\n".to_string();
    }
    let mut out = format!("budget recent ({} ledger rows", rows.len());
    if let Some(worker) = worker {
        out.push_str(&format!(", worker={worker}"));
    }
    if let Some(since) = since {
        out.push_str(&format!(", since={}", since.to_rfc3339()));
    }
    out.push_str("):\n");
    for row in rows {
        out.push_str(&format!(
            "  {}  ${:>10}  worker={}  action={}\n",
            row.charged_at,
            format_usd(row.usd_micros),
            row.worker_id,
            row.action_id
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    async fn store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    async fn insert_worker(store: &SessionStore, worker_id: &str, daily_cap: Option<u64>) {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS worker (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                vertical_id TEXT NOT NULL,
                persona_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                memory_scope TEXT NOT NULL,
                auto_approval_json TEXT NOT NULL,
                channels_json TEXT NOT NULL,
                created_at INTEGER NOT NULL
             )",
        )
        .execute(store.pool())
        .await
        .unwrap();
        let policy = AutoApprovalPolicy {
            enabled: true,
            daily_budget_usd_micros: daily_cap,
            ..Default::default()
        };
        sqlx::query(
            "INSERT INTO worker
             (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
              auto_approval_json, channels_json, created_at)
             VALUES (?, ?, 'demo', 'persona', 'role', 'workspace', ?, '[]', 1)",
        )
        .bind(worker_id)
        .bind(vec![0_u8; 16])
        .bind(serde_json::to_string(&policy).unwrap())
        .execute(store.pool())
        .await
        .unwrap();
    }

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, day, hour, 0, 0)
            .single()
            .unwrap()
    }

    #[tokio::test]
    async fn show_summary_calculates_rollups_and_daily_headroom() {
        let store = store().await;
        insert_worker(&store, "worker-a", Some(5_000_000)).await;
        worker_budget::record_charge(store.pool(), "a1", "worker-a", 1_500_000, at(6, 9))
            .await
            .unwrap();
        worker_budget::record_charge(store.pool(), "a2", "worker-a", 2_000_000, at(5, 9))
            .await
            .unwrap();
        worker_budget::record_charge(store.pool(), "old", "worker-a", 9_000_000, at(1, 9))
            .await
            .unwrap();

        let summary = budget_summary(&store, "worker-a", at(6, 12)).await.unwrap();
        assert_eq!(summary.today_usd_micros, 1_500_000);
        assert_eq!(summary.this_month_usd_micros, 12_500_000);
        assert_eq!(summary.remaining_today_usd, Some(3.5));
        assert_eq!(summary.remaining_month_usd, None);
        assert_eq!(summary.last_charges.len(), 3);
        let text = render_summary_text(&summary);
        assert!(text.contains("remaining today"));
        assert!(text.contains("action=a1"));
    }

    #[tokio::test]
    async fn recent_filters_by_worker_and_iso_since() {
        let store = store().await;
        insert_worker(&store, "worker-a", None).await;
        insert_worker(&store, "worker-b", None).await;
        worker_budget::record_charge(store.pool(), "a1", "worker-a", 1_000_000, at(1, 1))
            .await
            .unwrap();
        worker_budget::record_charge(store.pool(), "a2", "worker-a", 2_000_000, at(2, 1))
            .await
            .unwrap();
        worker_budget::record_charge(store.pool(), "b1", "worker-b", 3_000_000, at(3, 1))
            .await
            .unwrap();

        let since = parse_since(Some("2026-05-02T00:00:00Z")).unwrap();
        let rows = recent_charges(&store, 10, Some("worker-a"), since)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action_id, "a2");
        assert_eq!(rows[0].usd, 2.0);
        let json = serde_json::to_value(&rows).unwrap();
        assert_eq!(json[0]["worker_id"], "worker-a");
    }

    #[test]
    fn parse_since_rejects_non_iso_input() {
        let err = parse_since(Some("yesterday")).unwrap_err();
        assert!(err.to_string().contains("invalid --since"));
    }
}
