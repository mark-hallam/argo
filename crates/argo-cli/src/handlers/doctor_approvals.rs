//! `argo doctor approvals` — inspect approval requests expired by the
//! gateway expiry sweeper.
use crate::home::{db_path, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

const SWEEPER_DECIDED_BY: &str = "system:approval-expiry-sweeper";
type ExpiredApprovalTuple = (Vec<u8>, Vec<u8>, i64, Option<String>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExpiredApprovalRow {
    pub id: String,
    pub action_id: String,
    pub decided_at: String,
    pub decision: String,
}

pub async fn run(since: &str, json_out: bool) -> ArgoResult<()> {
    let window = parse_since(since)?;
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let rows = collect_from_store(&store, Utc::now(), window).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&rows)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
        return Ok(());
    }
    print_text(&rows, since);
    Ok(())
}

pub(crate) async fn collect_from_store(
    store: &SessionStore,
    now: DateTime<Utc>,
    window: Duration,
) -> ArgoResult<Vec<ExpiredApprovalRow>> {
    let cutoff = now.timestamp_micros() - duration_micros(window)?;
    let rows: Vec<ExpiredApprovalTuple> = sqlx::query_as(
        "SELECT id, action_id, decided_at, decision FROM approval_request \
         WHERE status = 'expired' AND decided_by = ? AND decided_at >= ? \
         ORDER BY decided_at DESC",
    )
    .bind(SWEEPER_DECIDED_BY)
    .bind(cutoff)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("approval doctor query: {e}")))?;

    rows.into_iter()
        .map(|(id, action_id, decided_at, decision)| {
            let id = Uuid::from_slice(&id)
                .map_err(|e| ArgoError::Provider(format!("approval id from db: {e}")))?
                .to_string();
            let decided_at = Utc
                .timestamp_micros(decided_at)
                .single()
                .ok_or_else(|| ArgoError::Provider("invalid approval decided_at".to_string()))?
                .to_rfc3339();
            Ok(ExpiredApprovalRow {
                id,
                action_id: hex::encode(action_id),
                decided_at,
                decision: decision.unwrap_or_default(),
            })
        })
        .collect()
}

pub(crate) fn parse_since(s: &str) -> ArgoResult<Duration> {
    let trimmed = s.trim();
    let (number, multiplier) = match trimmed.chars().last() {
        Some('h') => (&trimmed[..trimmed.len() - 1], 3_600),
        Some('m') => (&trimmed[..trimmed.len() - 1], 60),
        Some('s') => (&trimmed[..trimmed.len() - 1], 1),
        Some(c) if c.is_ascii_digit() => (trimmed, 1),
        _ => {
            return Err(ArgoError::Provider(format!(
                "invalid --since '{s}' (expected seconds, 30m, or 24h)"
            )));
        }
    };
    let value = number
        .parse::<u64>()
        .map_err(|e| ArgoError::Schema(format!("invalid --since '{s}' duration number: {e}")))?;
    Ok(Duration::from_secs(value.saturating_mul(multiplier)))
}

fn duration_micros(duration: Duration) -> ArgoResult<i64> {
    i64::try_from(duration.as_micros())
        .map_err(|_| ArgoError::Provider("approval doctor window is too large".to_string()))
}

fn print_text(rows: &[ExpiredApprovalRow], since: &str) {
    println!("argo doctor approvals — expired by timeout over {since}");
    if rows.is_empty() {
        println!("  (no expired approval rows)");
        return;
    }
    for row in rows {
        println!(
            "  - id={} action_id={} decided_at={} decision={}",
            row.id, row.action_id, row.decided_at, row.decision
        );
    }
}

#[cfg(test)]
#[path = "doctor_approvals_tests.rs"]
mod tests;
