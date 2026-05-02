//! `argo costs recent [--limit N] [--since-minutes M] [--json]`
//! — workspace-wide recent spend ledger entries. Mirrors the
//! gateway's `GET /costs/recent` (#625). Default limit 50,
//! capped 500. Same SQL the gateway runs.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(limit: u32, since_minutes: Option<i64>, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let pool = store.pool();
    let limit = limit.clamp(1, 500);
    let rows: Vec<(i64, Vec<u8>, i64, i64, String, String)> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT seq, session_id, charged_at_micros, amount_usd_micros, provider, model \
                 FROM spend_ledger WHERE charged_at_micros >= ? \
                 ORDER BY charged_at_micros DESC LIMIT ?",
            )
            .bind(cutoff)
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT seq, session_id, charged_at_micros, amount_usd_micros, provider, model \
                 FROM spend_ledger ORDER BY charged_at_micros DESC LIMIT ?",
            )
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("costs_recent: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(seq, sid_b, ts, amt, prov, model)| {
                let sid = uuid::Uuid::from_slice(sid_b)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| hex::encode(sid_b));
                serde_json::json!({
                    "seq": seq,
                    "session_id": sid,
                    "charged_at_micros": ts,
                    "amount_usd_micros": amt,
                    "provider": prov,
                    "model": model,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no spend in the last {m} minutes)"),
            None => println!("(no spend rows — run `argo chat` against a priced model first)"),
        }
        return Ok(());
    }
    println!("costs recent ({} ledger rows):", rows.len());
    for (seq, sid_b, _ts, amt, prov, model) in &rows {
        let sid = uuid::Uuid::from_slice(sid_b)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| hex::encode(sid_b));
        println!(
            "  [{seq:>5}] ${:>10.4}  {prov} / {model}  session={sid}",
            *amt as f64 / 1_000_000.0
        );
    }
    Ok(())
}
