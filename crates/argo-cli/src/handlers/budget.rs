//! `argo budget` — per-worker budget ledger reads.
use crate::cli::{BudgetAlertsCmd, BudgetCmd, BudgetOutputFormat};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::{QueryBuilder, Sqlite};

pub(crate) use super::budget_show::{micros_to_usd, BudgetSummary, ChargeRow};

const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/14__worker_budget.sql");

pub async fn run(cmd: BudgetCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    match cmd {
        BudgetCmd::Show {
            worker_id,
            json,
            format,
        } => {
            let summary = super::budget_show::budget_summary(&store, &worker_id, Utc::now()).await?;
            match format.resolve(json) {
                BudgetOutputFormat::Json => {
                    println!("{}", serde_json::to_string(&summary).unwrap_or_default());
                }
                BudgetOutputFormat::Csv => print!("{}", super::budget_csv::summary(&summary)),
                BudgetOutputFormat::Human => {
                    print!("{}", super::budget_show::render_summary_text(&summary));
                }
            }
        }
        BudgetCmd::Recent {
            limit,
            worker,
            since,
            json,
            format,
        } => {
            let since = parse_since(since.as_deref())?;
            let rows = recent_charges(&store, limit, worker.as_deref(), since).await?;
            match format.resolve(json) {
                BudgetOutputFormat::Json => {
                    println!("{}", serde_json::to_string(&rows).unwrap_or_default());
                }
                BudgetOutputFormat::Csv => print!("{}", super::budget_csv::charges(&rows)),
                BudgetOutputFormat::Human => {
                    print!(
                        "{}",
                        super::budget_show::render_recent_text(&rows, worker.as_deref(), since)
                    );
                }
            }
        }
        BudgetCmd::Anomalies {
            worker,
            since,
            limit,
            json,
            format,
        } => {
            let since = parse_since(since.as_deref())?;
            super::budget_anomalies::list(
                &store,
                worker.as_deref(),
                since,
                limit,
                format.resolve(json),
            )
            .await?;
        }
        BudgetCmd::AnomalyTick { json } => {
            super::budget_anomalies::tick(&store, Utc::now(), json).await?;
        }
        BudgetCmd::BySkill {
            skill_name,
            history,
            since,
            limit,
            vertical,
            json,
        } => {
            super::budget_by_skill::run(
                skill_name.as_deref(),
                history,
                since.as_deref(),
                limit,
                vertical.as_deref(),
                json,
            )
            .await?;
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
                format,
            } => match format.resolve(json) {
                BudgetOutputFormat::Csv => {
                    let since = super::budget_watch::parse_since(since.as_deref())?;
                    let rows =
                        super::budget_watch::list_alerts(&store, worker.as_deref(), since, limit)
                            .await?;
                    print!("{}", super::budget_csv::alerts(&rows));
                }
                BudgetOutputFormat::Json | BudgetOutputFormat::Human => {
                    super::budget_alerts_list::run(
                        worker.as_deref(),
                        since.as_deref(),
                        limit,
                        json || format == BudgetOutputFormat::Json,
                    )
                    .await?;
                }
            },
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
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

pub(crate) async fn recent_charges(
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

pub(crate) async fn ensure_budget_schema(store: &SessionStore) -> ArgoResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::{worker_budget, AutoApprovalPolicy};
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

        let summary = super::super::budget_show::budget_summary(&store, "worker-a", at(6, 12))
            .await
            .unwrap();
        assert_eq!(summary.today_usd_micros, 1_500_000);
        assert_eq!(summary.this_month_usd_micros, 12_500_000);
        assert_eq!(summary.remaining_today_usd, Some(3.5));
        assert_eq!(summary.remaining_month_usd, None);
        assert_eq!(summary.last_charges.len(), 3);
        let text = super::super::budget_show::render_summary_text(&summary);
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
