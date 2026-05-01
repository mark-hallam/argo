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
                )
                .await?;
        }
    }
    println!("argo forge runs abort-all: aborted {aborted} run(s)");
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
        )
        .await?;
    println!("argo forge runs abort: {raw} → status=aborted");
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
    Err(ArgoError::Provider(format!("invalid forge run id '{raw}'")))
}

fn ts(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "(?)".to_string())
}
