//! `argo costs` — inspect or set hard $$$ ceilings, plus a quick
//! actual-spend roll-up read straight from the spend ledger.
use crate::cli::CostsCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;

pub async fn run(cmd: Option<CostsCmd>) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml"))?;
    match cmd {
        None => super::costs_show::show(&cwd, &cfg, "text").await,
        Some(CostsCmd::Show {
            json,
            by_model,
            by_session,
            format,
        }) => {
            let pick = if json { "json".to_string() } else { format };
            if by_model {
                super::costs_show::show_grouped(&cwd, "by_model", &pick).await
            } else if by_session {
                super::costs_show::show_grouped(&cwd, "by_session", &pick).await
            } else {
                super::costs_show::show(&cwd, &cfg, &pick).await
            }
        }
        Some(CostsCmd::Export { out, since_minutes }) => {
            export_ledger(&cwd, out.as_deref(), since_minutes).await
        }
        Some(CostsCmd::Alert {
            threshold_usd,
            json,
        }) => alert_over_threshold(&cwd, threshold_usd, json).await,
        Some(CostsCmd::Total { json }) => total(&cwd, json).await,
        Some(CostsCmd::ByDay { days, json }) => {
            super::costs_by_day::run(days, json).await
        }
        Some(CostsCmd::ByModel { json }) => super::costs_by_model::run(json).await,
        Some(CostsCmd::ByProvider { json }) => super::costs_by_provider::run(json).await,
        Some(CostsCmd::BySession { json }) => super::costs_by_session::run(json).await,
        Some(CostsCmd::Recent {
            limit,
            since_minutes,
            json,
        }) => super::costs_recent::run(limit, since_minutes, json).await,
        Some(CostsCmd::Anomalies {
            since,
            limit,
            format,
            tenant,
        }) => {
            super::costs_anomalies::run(since.as_deref(), limit, format, tenant.as_deref()).await
        }
        Some(CostsCmd::Forecast { .. }) => unreachable!("Forecast dispatched in main.rs"),
        Some(CostsCmd::Cap {
            session,
            day,
            month,
            usd,
            reset,
        }) => {
            if !session && !day && !month {
                return Err(ArgoError::Provider(
                    "specify exactly one of --session / --day / --month".into(),
                ));
            }
            let scope = if session {
                "session"
            } else if day {
                "day"
            } else {
                "month"
            };
            if reset {
                return reset_cap(&cwd, scope);
            }
            let usd = usd.ok_or_else(|| {
                ArgoError::Provider("provide a <usd> value or pass --reset".into())
            })?;
            let micros = (usd * 1_000_000.0).round() as u64;
            apply_cap(&cwd, scope, micros, usd)
        }
    }
}

/// Remove the per-scope cap entry from `argo.toml`. The config
/// loader falls back to the compiled-in default for that scope.
fn reset_cap(cwd: &std::path::Path, scope: &str) -> ArgoResult<()> {
    let path = cwd.join("argo.toml");
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let mut value: toml::Value = toml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let key = format!("default_spend_per_{scope}_usd_micros");
    let removed = value
        .as_table_mut()
        .and_then(|t| t.get_mut("budget"))
        .and_then(|b| b.as_table_mut())
        .and_then(|b| b.remove(&key))
        .is_some();
    if !removed {
        println!("argo costs cap: per-{scope} cap was already at the default");
        return Ok(());
    }
    let serialized = toml::to_string_pretty(&value)
        .map_err(|e| ArgoError::Provider(format!("serialize argo.toml: {e}")))?;
    std::fs::write(&path, serialized)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    println!(
        "argo costs cap: per-{scope} cap reset to default in {}",
        path.display()
    );
    Ok(())
}

/// Persist a new spend cap to `argo.toml` under `[budget]`. Reads
/// the file, mutates `default_spend_per_<scope>_usd_micros` in
/// place, and writes back. Comments are NOT preserved (toml crate
/// limitation); for cap-on-bootstrap workflows that's acceptable.
fn apply_cap(cwd: &std::path::Path, scope: &str, micros: u64, usd: f64) -> ArgoResult<()> {
    let path = cwd.join("argo.toml");
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let mut value: toml::Value = toml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let key = format!("default_spend_per_{scope}_usd_micros");
    let table = value
        .as_table_mut()
        .ok_or_else(|| ArgoError::Provider("argo.toml is not a table".into()))?
        .entry("budget")
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .ok_or_else(|| ArgoError::Provider("[budget] is not a table".into()))?;
    table.insert(key.clone(), toml::Value::Integer(micros as i64));
    let serialized = toml::to_string_pretty(&value)
        .map_err(|e| ArgoError::Provider(format!("serialize argo.toml: {e}")))?;
    std::fs::write(&path, serialized)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    println!(
        "argo costs cap: per-{scope} cap set to ${usd} ({micros} usd-micros) in {}",
        path.display()
    );
    println!("  daemon hot-reloads on SIGHUP; CLI picks it up on next invocation.");
    Ok(())
}

#[derive(Default)]
pub(super) struct GlobalSpend {
    pub(super) today: u64,
    pub(super) this_month: u64,
    pub(super) all_time: u64,
}

pub(super) fn micros_to_usd(m: u64) -> f64 {
    m as f64 / 1_000_000.0
}

/// `argo costs total [--json]` — print just the today / mtd /
/// all-time spend roll-up. Mirrors the gateway's
/// `GET /costs/total` (#405). Defaults to the formatted
/// `$X.XX` view; `--json` emits `{today, this_month, all_time}`
/// in usd_micros for scripting.
async fn total(cwd: &std::path::Path, json: bool) -> ArgoResult<()> {
    let spent = load_global_spend(cwd).await?;
    if json {
        let payload = serde_json::json!({
            "today":      spent.today,
            "this_month": spent.this_month,
            "all_time":   spent.all_time,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("today:      ${:>10.2}", micros_to_usd(spent.today));
        println!("this month: ${:>10.2}", micros_to_usd(spent.this_month));
        println!("all time:   ${:>10.2}", micros_to_usd(spent.all_time));
    }
    Ok(())
}

/// Roll the spend ledger up across all tenants for "today" (UTC),
/// month-to-date, and all-time totals. Now a thin wrapper over
/// `SessionStore::load_global_spend` — the SQL lives in
/// `argo-store::spend` so the TUI cost overlay can call it
/// without re-implementing the queries.
pub(super) async fn load_global_spend(cwd: &std::path::Path) -> ArgoResult<GlobalSpend> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let snap = store.load_global_spend().await?;
    Ok(GlobalSpend {
        today: snap.today,
        this_month: snap.this_month,
        all_time: snap.all_time,
    })
}

/// Stream every spend-ledger row as NDJSON (one JSON object per
/// line). Default destination is stdout; `--out <path>` writes to
/// a file. Each row is `{tenant_id, session_id, charged_at_micros,
/// amount_usd_micros, provider, model}` (uuids hex-encoded for
/// portability).
async fn export_ledger(
    cwd: &std::path::Path,
    out: Option<&str>,
    since_minutes: Option<i64>,
) -> ArgoResult<()> {
    use std::io::Write;
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    type SpendRow = (Vec<u8>, Vec<u8>, i64, i64, String, String);
    let cutoff = since_minutes.map(|m| {
        (Utc::now() - chrono::Duration::minutes(m)).timestamp_micros()
    });
    let rows: Vec<SpendRow> = match cutoff {
        Some(c) => sqlx::query_as(
            "SELECT tenant_id, session_id, charged_at_micros, amount_usd_micros, provider, model \
             FROM spend_ledger WHERE charged_at_micros >= ? ORDER BY charged_at_micros ASC",
        )
        .bind(c)
        .fetch_all(store.pool())
        .await,
        None => sqlx::query_as(
            "SELECT tenant_id, session_id, charged_at_micros, amount_usd_micros, provider, model \
             FROM spend_ledger ORDER BY charged_at_micros ASC",
        )
        .fetch_all(store.pool())
        .await,
    }
    .map_err(|e| ArgoError::Provider(format!("export_ledger: {e}")))?;

    let mut writer: Box<dyn Write> = match out {
        Some(p) => {
            let f = std::fs::File::create(p)
                .map_err(|e| ArgoError::Io(format!("create {p}: {e}")))?;
            Box::new(std::io::BufWriter::new(f))
        }
        None => Box::new(std::io::BufWriter::new(std::io::stdout())),
    };
    for (tenant, session, ts, micros, provider, model) in &rows {
        let line = serde_json::json!({
            "tenant_id": hex::encode(tenant),
            "session_id": hex::encode(session),
            "charged_at_micros": ts,
            "amount_usd_micros": u64::try_from(*micros).unwrap_or(0),
            "provider": provider,
            "model": model,
        });
        let serialized = serde_json::to_string(&line)
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
        writeln!(writer, "{serialized}")
            .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    }
    writer
        .flush()
        .map_err(|e| ArgoError::Provider(format!("flush: {e}")))?;
    if let Some(p) = out {
        eprintln!("costs export: wrote {} rows to {p}", rows.len());
    }
    Ok(())
}

/// List sessions whose total spend exceeds `threshold_usd`, sorted
/// by spend desc. Reads the spend ledger once (one GROUP BY query
/// + threshold filter in SQL).
async fn alert_over_threshold(
    cwd: &std::path::Path,
    threshold_usd: f64,
    json: bool,
) -> ArgoResult<()> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let threshold_micros = (threshold_usd * 1_000_000.0).round().max(0.0) as i64;
    let rows: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT session_id, COALESCE(SUM(amount_usd_micros), 0) AS total \
         FROM spend_ledger GROUP BY session_id HAVING total > ? \
         ORDER BY total DESC",
    )
    .bind(threshold_micros)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("alert query: {e}")))?;
    if json {
        for (sid, total) in &rows {
            let line = serde_json::json!({
                "session_id": hex::encode(sid),
                "spent_usd_micros": u64::try_from(*total).unwrap_or(0),
                "threshold_usd": threshold_usd,
            });
            println!(
                "{}",
                serde_json::to_string(&line)
                    .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
            );
        }
        return Ok(());
    }
    if rows.is_empty() {
        println!(
            "no sessions over ${threshold_usd:.4} (queried {} ledger rows)",
            ledger_row_count(&store).await.unwrap_or(0)
        );
        return Ok(());
    }
    println!(
        "sessions over ${threshold_usd:.4} ({} match(es)):",
        rows.len()
    );
    for (sid, total) in &rows {
        println!(
            "  ${:>10.4}  {}",
            *total as f64 / 1_000_000.0,
            hex::encode(sid)
        );
    }
    Ok(())
}

async fn ledger_row_count(store: &SessionStore) -> ArgoResult<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM spend_ledger")
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("count: {e}")))
}
