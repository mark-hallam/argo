//! Forge run admin ops — show / abort / abort-all. Lifted out of
//! `forge.rs` so that file stays under the 500-LOC budget as new
//! count/list filters land. All three handlers share the same
//! `parse_run_id` helper + the audit-on-abort wiring.

use super::forge_subject::open_store;
use argo_audit::{AuditKind, AuditSink};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub(super) async fn show_run(raw: &str) -> ArgoResult<()> {
    let store = open_store().await?;
    let id = parse_run_id(raw)?;
    let row = store
        .get_forge_run(id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no forge run with id {raw}")))?;
    let started = ts(row.started_at);
    let ended = row
        .ended_at
        .map(ts)
        .unwrap_or_else(|| "(running)".to_string());
    println!("forge run {}", hex::encode(&row.id));
    println!("  skill:        {}", row.skill_name);
    println!("  status:       {}", row.status);
    println!("  base:         {}", row.base_version);
    println!(
        "  candidate:    {}",
        row.candidate_version.as_deref().unwrap_or("(none)")
    );
    println!("  iterations:   {}", row.iterations);
    println!(
        "  spend:        ${:.2}",
        row.spend_usd_micros as f64 / 1_000_000.0
    );
    println!(
        "  final_score:  {}",
        row.final_score
            .map(|s| format!("{s:.3}"))
            .unwrap_or_else(|| "-".into())
    );
    println!("  started_at:   {started}");
    println!("  ended_at:     {ended}");
    Ok(())
}

/// `argo forge runs abort-all` — flip every running forge_run to
/// `aborted`. Mirrors the gateway's
/// `POST /forge/runs/abort-all`. Useful as an operator
/// emergency-stop / fresh-slate before a workspace migration.
pub(super) async fn abort_all_runs() -> ArgoResult<()> {
    let store = Arc::new(open_store().await?);
    let rows = store.list_forge_runs(500).await?;
    let mut aborted: u64 = 0;
    for r in rows.into_iter().filter(|r| r.status == "running") {
        let id = match uuid::Uuid::from_slice(&r.id) {
            Ok(u) => u,
            Err(_) => continue,
        };
        if let Ok(n) = store.abort_forge_run(id).await {
            aborted += n;
            AuditSink::new(Arc::clone(&store))
                .write(
                    None,
                    AuditKind::ForgeRunEnd,
                    serde_json::json!({
                        "kind": "abort",
                        "run_id": id.to_string(),
                        "by": "cli",
                        "via": "abort-all",
                    }),
                    None,
                )
                .await?;
        }
    }
    println!("argo forge runs abort-all: aborted {aborted} run(s)");
    Ok(())
}

/// `argo forge runs sweep [--older-than-minutes N]` — recover
/// from a gateway crash mid-optimize. Walks `forge_run` rows
/// in `running` status whose `started_at` is older than the
/// cutoff and flips them to `failed`. Writes a `ForgeRunEnd`
/// audit row per swept run with `kind=sweep`.
pub(super) async fn sweep_stalled(older_than_minutes: i64) -> ArgoResult<()> {
    let store = Arc::new(open_store().await?);
    let cutoff_micros = (chrono::Utc::now() - chrono::Duration::minutes(older_than_minutes))
        .timestamp_micros();
    // Find the rows we're about to sweep so we can write
    // per-row audit rows. The store call below does the actual
    // UPDATE — we just observe first for the audit trail.
    let candidates = store
        .list_forge_runs(10_000)
        .await?
        .into_iter()
        .filter(|r| r.status == "running" && r.started_at < cutoff_micros)
        .collect::<Vec<_>>();
    let swept = store.sweep_stalled_forge_runs(cutoff_micros).await?;
    for r in &candidates {
        if let Ok(id) = uuid::Uuid::from_slice(&r.id) {
            AuditSink::new(Arc::clone(&store))
                .write(
                    None,
                    AuditKind::ForgeRunEnd,
                    serde_json::json!({
                        "kind": "sweep",
                        "run_id": id.to_string(),
                        "by": "cli",
                        "older_than_minutes": older_than_minutes,
                    }),
                    None,
                )
                .await?;
        }
    }
    println!(
        "argo forge runs sweep: marked {swept} stalled run(s) as failed (older than {older_than_minutes}m)"
    );
    Ok(())
}

pub(super) async fn abort_run(raw: &str) -> ArgoResult<()> {
    let store = Arc::new(open_store().await?);
    let id = parse_run_id(raw)?;
    let affected = store.abort_forge_run(id).await?;
    if affected == 0 {
        return Err(ArgoError::Provider(format!("no forge run with id {raw}")));
    }
    AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "kind": "abort",
                "run_id": id.to_string(),
                "by": "cli",
            }),
            None,
        )
        .await?;
    println!("argo forge runs abort: {raw} → status=aborted");
    Ok(())
}

/// `argo forge runs payload <id>` — mirror of the gateway's
/// `GET /forge/runs/<id>/payload`. Prints the queued payload
/// (skill / examples / iterations / budget_usd) for a run.
pub(super) async fn show_payload(raw: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let id = parse_run_id(raw)?;
    let payload_str = store
        .get_forge_queue_payload(id)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "no forge_queue payload for run {raw} (was it created with ?in_process=true?)"
            ))
        })?;
    let parsed: serde_json::Value =
        serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
    if json {
        let payload = serde_json::json!({
            "run_id": id.to_string(),
            "payload": parsed,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("argo forge runs payload: {raw}");
        println!(
            "{}",
            serde_json::to_string_pretty(&parsed).unwrap_or(payload_str)
        );
    }
    Ok(())
}

/// `argo forge runs requeue <id>` — mirror of the gateway's
/// `POST /forge/runs/<id>/requeue`. Copies the source row's
/// queued payload into a fresh queued run; daemon picks it
/// up on its next tick.
pub(super) async fn requeue_run(raw: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let id = parse_run_id(raw)?;
    let new_id = store.requeue_forge_run(id).await?.ok_or_else(|| {
        ArgoError::Provider(format!(
            "no forge_queue payload for run {raw} (was it created with ?in_process=true?)"
        ))
    })?;
    if json {
        let payload = serde_json::json!({
            "requeued": true,
            "source_id": id.to_string(),
            "run_id": new_id.to_string(),
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("argo forge runs requeue: {raw} → new run {new_id} (status=queued)");
    }
    Ok(())
}

/// Resolve a forge-run id from the user's input. Accepts either a
/// full UUID or an 8-hex-char short prefix (matching what
/// `argo forge runs list` prints).
fn parse_run_id(raw: &str) -> ArgoResult<uuid::Uuid> {
    if let Ok(u) = uuid::Uuid::parse_str(raw) {
        return Ok(u);
    }
    if raw.len() >= 8 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ArgoError::Provider(format!(
            "short prefix '{raw}' — pass the full uuid (see `argo forge runs list`)"
        )));
    }
    Err(ArgoError::Schema(format!("invalid forge run id '{raw}'")))
}

fn ts(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "(?)".to_string())
}

/// `argo forge runs audit <id>` — mirror of the gateway's
/// `GET /forge/runs/<id>/audit` (#634). Audit rows linked to a
/// forge run via the payload's `$.run_id` field, ordered seq ASC.
pub(super) async fn show_audit(raw: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let id = parse_run_id(raw)?;
    let id_str = id.to_string();
    let rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT seq, kind, payload_json, created_at FROM audit_record \
         WHERE json_extract(payload_json, '$.run_id') = ? \
         ORDER BY seq ASC",
    )
    .bind(&id_str)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("forge_run_audit: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(seq, kind, payload_json, ts)| {
                serde_json::json!({
                    "seq": seq,
                    "kind": kind,
                    "payload": serde_json::from_str::<serde_json::Value>(payload_json)
                        .unwrap_or(serde_json::Value::Null),
                    "created_at_micros": ts,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "run_id": id_str,
                "rows": arr,
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no audit rows for forge run {raw})");
        return Ok(());
    }
    println!("audit rows for forge run {raw} ({} rows):", rows.len());
    for (seq, kind, _payload, ts) in &rows {
        println!("  [{seq:>4}] {kind:<24} ts={ts}");
    }
    Ok(())
}
