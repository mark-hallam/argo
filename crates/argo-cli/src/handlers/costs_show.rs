//! `argo costs show` formatters — ceilings + spend roll-up, plus the
//! grouped (`--by-model` / `--by-session`) views. Extracted from
//! `handlers/costs.rs` to keep the dispatch file under the 500-LOC
//! ceiling.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use super::costs::{load_global_spend, micros_to_usd};

pub(super) async fn show(
    cwd: &std::path::Path,
    cfg: &ArgoConfig,
    format: &str,
) -> ArgoResult<()> {
    let ceilings = cfg.budget.to_cost_ceilings();
    let spent = load_global_spend(cwd).await.unwrap_or_default();
    match format {
        "json" => {
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
        "markdown" => {
            println!("# Costs");
            println!();
            println!("## Ceilings (USD)");
            println!();
            for (label, val) in [
                ("session", ceilings.per_session_usd_micros),
                ("day", ceilings.per_day_usd_micros),
                ("month", ceilings.per_month_usd_micros),
            ] {
                match val {
                    Some(v) => println!("- **per-{label}**: ${:.2}", v as f64 / 1_000_000.0),
                    None => println!("- **per-{label}**: _(unlimited)_"),
                }
            }
            println!(
                "- **notify_at_pct**: {}",
                ceilings
                    .notify_at_pct
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!();
            println!("## Spent so far (USD, all tenants)");
            println!();
            println!("- **Today**: ${:.2}", micros_to_usd(spent.today));
            println!("- **This month**: ${:.2}", micros_to_usd(spent.this_month));
            println!("- **All time**: ${:.2}", micros_to_usd(spent.all_time));
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
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

/// Group spend rows by `(provider, model)` or `session_id` and
/// print the top 20 spenders. Cheap one-shot SQL — no in-memory
/// rollup. `mode` is `"by_model"` or `"by_session"`.
pub(super) async fn show_grouped(
    cwd: &std::path::Path,
    mode: &str,
    format: &str,
) -> ArgoResult<()> {
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
    match format {
        "json" => {
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
        "markdown" => {
            let title = if mode == "by_model" {
                "Spend by model (USD, top 20)"
            } else {
                "Spend by session (USD, top 20)"
            };
            println!("# {title}");
            println!();
            if rows.is_empty() {
                println!("_(no spend rows yet — run `argo chat` against a priced model first)_");
                return Ok(());
            }
            println!("| bucket | total (USD) |\n| --- | ---: |");
            for (bucket, total) in &rows {
                println!("| `{bucket}` | ${:.4} |", *total as f64 / 1_000_000.0);
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
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
