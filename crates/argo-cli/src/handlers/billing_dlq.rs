//! `argo billing dlq <list|resolve>` — inspect or close out
//! dead-lettered Stripe webhook events (Stream
//! `codex-billing-webhook-resilience`).

use crate::cli::BillingDlqCmd;
use crate::handlers::action_history::parse_duration;
use argo_store::{billing_dlq_query, SessionStore};
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::json;

/// Dispatch `argo billing dlq …`. Admin-only by convention.
pub async fn dispatch(cmd: BillingDlqCmd) -> ArgoResult<()> {
    tracing::info!(target: "billing::dlq", "admin dlq invoked");
    let store = open_store().await?;
    match cmd {
        BillingDlqCmd::List {
            since,
            include_resolved,
            limit,
            json,
        } => list(&store, since.as_deref(), include_resolved, limit, json).await,
        BillingDlqCmd::Resolve {
            event_id,
            note,
            json,
        } => resolve(&store, &event_id, note.as_deref(), json).await,
    }
}

async fn list(
    store: &SessionStore,
    since: Option<&str>,
    include_resolved: bool,
    limit: i64,
    json_out: bool,
) -> ArgoResult<()> {
    let now = Utc::now();
    let since_secs = match since {
        Some(s) => Some((now - parse_duration(s)?).timestamp()),
        None => None,
    };
    let rows = billing_dlq_query::list_dead_letters(
        store,
        since_secs,
        include_resolved,
        limit,
    )
    .await?;
    if json_out {
        let payload: Vec<_> = rows.iter().map(entry_to_json).collect();
        println!(
            "{}",
            serde_json::to_string(&payload)
                .map_err(|e| ArgoError::Provider(format!("dlq list json: {e}")))?,
        );
    } else if rows.is_empty() {
        println!("(no DLQ entries)");
    } else {
        for entry in &rows {
            let resolved = entry
                .resolved_at
                .map(|t| format!("resolved@{t}"))
                .unwrap_or_else(|| "outstanding".to_string());
            println!(
                "{}\t{}\tretries={}\t{}\t{}",
                entry.event_id, entry.event_type, entry.retry_count, resolved, entry.failure_reason,
            );
        }
        println!("({} entries)", rows.len());
    }
    Ok(())
}

async fn resolve(
    store: &SessionStore,
    event_id: &str,
    note: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    let now = Utc::now();
    let updated = billing_dlq_query::mark_resolved(store, event_id, now).await?;
    if let Some(text) = note {
        tracing::info!(
            target: "billing::dlq",
            event_id, note = text,
            "operator-supplied resolve note",
        );
    }
    if json_out {
        println!(
            "{}",
            json!({
                "event_id": event_id,
                "resolved": updated,
                "note": note,
            })
        );
    } else if updated {
        println!("{event_id}: resolved");
    } else {
        println!("{event_id}: no outstanding DLQ row (already resolved or never existed)");
    }
    Ok(())
}

fn entry_to_json(entry: &billing_dlq_query::DeadLetterEntry) -> serde_json::Value {
    json!({
        "event_id": entry.event_id,
        "event_type": entry.event_type,
        "tenant_id": entry.tenant_id.map(|t| t.to_string()),
        "failed_at": entry.failed_at,
        "failure_reason": entry.failure_reason,
        "retry_count": entry.retry_count,
        "resolved_at": entry.resolved_at,
    })
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let db = cwd.join(".skycot").join("sessions.db");
    let store = SessionStore::open(db.to_string_lossy().as_ref()).await?;
    store.migrate().await?;
    Ok(store)
}
