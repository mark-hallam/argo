//! `argo costs` — inspect or set hard $$$ ceilings, plus a quick
//! actual-spend roll-up read straight from the spend ledger.
use crate::cli::CostsCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{Datelike, TimeZone, Utc};

pub async fn run(cmd: Option<CostsCmd>) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml"))?;
    match cmd {
        None => show(&cwd, &cfg, false).await,
        Some(CostsCmd::Show {
            json,
            by_model,
            by_session,
        }) => {
            if by_model {
                show_grouped(&cwd, "by_model", json).await
            } else if by_session {
                show_grouped(&cwd, "by_session", json).await
            } else {
                show(&cwd, &cfg, json).await
            }
        }
        Some(CostsCmd::Export { out }) => export_ledger(&cwd, out.as_deref()).await,
        Some(CostsCmd::Cap {
            session,
            day,
            month,
            usd,
        }) => {
            // No flag selected → print help-ish message instead of
            // silently picking a scope.
            if !session && !day && !month {
                return Err(ArgoError::Provider(
                    "specify exactly one of --session / --day / --month".into(),
                ));
            }
            let micros = (usd * 1_000_000.0).round() as u64;
            let scope = if session {
                "session"
            } else if day {
                "day"
            } else {
                "month"
            };
            println!("would set per-{scope} cap to ${usd} ({micros} usd-micros);");
            println!(
                "  edit `argo.toml` directly under [budget]; the daemon hot-reloads on SIGHUP."
            );
            Ok(())
        }
    }
}

async fn show(cwd: &std::path::Path, cfg: &ArgoConfig, json: bool) -> ArgoResult<()> {
    let ceilings = cfg.budget.to_cost_ceilings();
    let spent = load_global_spend(cwd).await.unwrap_or_default();
    if json {
        let payload = serde_json::json!({
            "per_session_usd_micros": ceilings.per_session_usd_micros,
            "per_day_usd_micros":     ceilings.per_day_usd_micros,
            "per_month_usd_micros":   ceilings.per_month_usd_micros,
            "notify_at_pct":          ceilings.notify_at_pct,
            "spent_usd_micros": {
                "today":     spent.today,
                "this_month": spent.this_month,
                "all_time":  spent.all_time,
            },
        });
        let serialized = serde_json::to_string(&payload)
            .map_err(|e| ArgoError::Provider(format!("serialize costs: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
    println!("cost ceilings (USD):");
    for (label, val) in [
        ("session", ceilings.per_session_usd_micros),
        ("day", ceilings.per_day_usd_micros),
        ("month", ceilings.per_month_usd_micros),
    ] {
        match val {
            Some(v) => println!("  per-{label:<7} ${:>10.2}", v as f64 / 1_000_000.0),
            None => println!("  per-{label:<7} (unlimited)"),
        }
    }
    println!(
        "notify_at_pct: {}",
        ceilings
            .notify_at_pct
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("spent so far (USD, all tenants):");
    println!("  today:      ${:>10.2}", micros_to_usd(spent.today));
    println!("  this month: ${:>10.2}", micros_to_usd(spent.this_month));
    println!("  all time:   ${:>10.2}", micros_to_usd(spent.all_time));
    Ok(())
}

#[derive(Default)]
struct GlobalSpend {
    today: u64,
    this_month: u64,
    all_time: u64,
}

fn micros_to_usd(m: u64) -> f64 {
    m as f64 / 1_000_000.0
}

/// Roll the spend ledger up across all tenants for "today" (UTC),
/// month-to-date, and all-time totals. Skipped silently when the
/// store / spend table doesn't exist yet — `argo costs show` is
/// useful even on a brand-new workspace.
async fn load_global_spend(cwd: &std::path::Path) -> ArgoResult<GlobalSpend> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let now = Utc::now();
    let day_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .map(|t| t.timestamp_micros())
        .unwrap_or(0);
    let month_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .map(|t| t.timestamp_micros())
        .unwrap_or(0);
    let pool = store.pool();
    let today: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger WHERE charged_at_micros >= ?",
    )
    .bind(day_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("today sum: {e}")))?;
    let this_month: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger WHERE charged_at_micros >= ?",
    )
    .bind(month_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("month sum: {e}")))?;
    let all_time: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger")
            .fetch_one(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("all-time sum: {e}")))?;
    Ok(GlobalSpend {
        today: u64::try_from(today).unwrap_or(0),
        this_month: u64::try_from(this_month).unwrap_or(0),
        all_time: u64::try_from(all_time).unwrap_or(0),
    })
}

/// Stream every spend-ledger row as NDJSON (one JSON object per
/// line). Default destination is stdout; `--out <path>` writes to
/// a file. Each row is `{tenant_id, session_id, charged_at_micros,
/// amount_usd_micros, provider, model}` (uuids hex-encoded for
/// portability).
async fn export_ledger(cwd: &std::path::Path, out: Option<&str>) -> ArgoResult<()> {
    use std::io::Write;
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    type SpendRow = (Vec<u8>, Vec<u8>, i64, i64, String, String);
    let rows: Vec<SpendRow> = sqlx::query_as(
        "SELECT tenant_id, session_id, charged_at_micros, amount_usd_micros, provider, model \
         FROM spend_ledger ORDER BY charged_at_micros ASC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("export_ledger: {e}")))?;

    let mut writer: Box<dyn Write> = match out {
        Some(p) => {
            let f = std::fs::File::create(p)
                .map_err(|e| ArgoError::Provider(format!("create {p}: {e}")))?;
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

/// Group spend rows by `(provider, model)` or `session_id` and
/// print the top 20 spenders. Cheap one-shot SQL — no in-memory
/// rollup. `mode` is `"by_model"` or `"by_session"`.
async fn show_grouped(cwd: &std::path::Path, mode: &str, json: bool) -> ArgoResult<()> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let sql = match mode {
        "by_model" => {
            "SELECT provider || ' / ' || model AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY provider, model ORDER BY total DESC LIMIT 20"
        }
        "by_session" => {
            "SELECT lower(hex(session_id)) AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY session_id ORDER BY total DESC LIMIT 20"
        }
        _ => unreachable!("mode must be by_model or by_session"),
    };
    let rows: Vec<(String, i64)> = sqlx::query_as(sql)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("group query: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(b, t)| {
                serde_json::json!({"bucket": b, "spent_usd_micros": u64::try_from(*t).unwrap_or(0)})
            })
            .collect();
        let out = serde_json::to_string(&serde_json::json!({mode: arr}))
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
        println!("{out}");
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no spend rows yet — run `argo chat` against a priced model first)");
        return Ok(());
    }
    let header = if mode == "by_model" {
        "spend by model (USD, top 20):"
    } else {
        "spend by session (USD, top 20):"
    };
    println!("{header}");
    for (bucket, total) in rows {
        println!("  ${:>10.4}  {bucket}", total as f64 / 1_000_000.0);
    }
    Ok(())
}
