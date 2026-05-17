//! `argo billing replay` — re-apply dead-lettered Stripe events from
//! their stored `raw_payload`. No Stripe API call (Stream
//! `codex-billing-webhook-resilience`).

use crate::cli::BillingReplayArgs;
use crate::handlers::action_history::parse_duration;
use argo_billing::{replay_event, replay_since, ReplayOutcome};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::json;

/// Dispatch `argo billing replay`. Admin-only by convention; the CLI
/// runs locally as the workspace operator.
pub async fn run(args: BillingReplayArgs) -> ArgoResult<()> {
    let store = open_store().await?;
    let now = Utc::now();
    tracing::info!(
        target: "billing::replay",
        event_id = args.event_id.as_deref(),
        since = args.since.as_deref(),
        limit = args.limit,
        "admin replay invoked",
    );
    match (args.event_id.as_deref(), args.since.as_deref()) {
        (Some(id), None) => replay_one(&store, id, now, args.json).await,
        (None, Some(window)) => replay_window(&store, window, args.limit, now, args.json).await,
        (Some(_), Some(_)) => Err(ArgoError::Schema(
            "pass exactly one of --event-id or --since".into(),
        )),
        (None, None) => Err(ArgoError::Schema(
            "either --event-id or --since is required".into(),
        )),
    }
}

async fn replay_one(
    store: &SessionStore,
    event_id: &str,
    now: chrono::DateTime<Utc>,
    json_out: bool,
) -> ArgoResult<()> {
    let outcome = replay_event(store, event_id, now).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&render_outcome(event_id, &outcome))
                .map_err(|e| ArgoError::Provider(format!("replay json: {e}")))?,
        );
    } else {
        println!("{}: {}", event_id, label(&outcome));
    }
    Ok(())
}

async fn replay_window(
    store: &SessionStore,
    window: &str,
    limit: i64,
    now: chrono::DateTime<Utc>,
    json_out: bool,
) -> ArgoResult<()> {
    let dur = parse_duration(window)?;
    let since = now - dur;
    let results = replay_since(store, since, limit, now).await?;
    if json_out {
        let rendered: Vec<_> = results
            .iter()
            .map(|(id, o)| render_outcome(id, o))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&rendered)
                .map_err(|e| ArgoError::Provider(format!("replay json: {e}")))?,
        );
    } else if results.is_empty() {
        println!("(no DLQ entries since {window})");
    } else {
        for (id, outcome) in &results {
            println!("{}: {}", id, label(outcome));
        }
        println!("replayed {} entries", results.len());
    }
    Ok(())
}

fn render_outcome(event_id: &str, outcome: &ReplayOutcome) -> serde_json::Value {
    match outcome {
        ReplayOutcome::Replayed {
            rows_touched,
            past_due_warning,
        } => json!({
            "event_id": event_id,
            "outcome": "replayed",
            "rows_touched": rows_touched,
            "past_due_warning": past_due_warning,
        }),
        ReplayOutcome::ResolvedAsIgnored => json!({
            "event_id": event_id,
            "outcome": "resolved_as_ignored",
        }),
        ReplayOutcome::NotFound => json!({
            "event_id": event_id,
            "outcome": "not_found",
        }),
    }
}

fn label(outcome: &ReplayOutcome) -> &'static str {
    match outcome {
        ReplayOutcome::Replayed { .. } => "replayed",
        ReplayOutcome::ResolvedAsIgnored => "resolved (classifier ignored)",
        ReplayOutcome::NotFound => "no DLQ row (already resolved or never failed)",
    }
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let db = cwd.join(".skycot").join("sessions.db");
    let store = SessionStore::open(db.to_string_lossy().as_ref()).await?;
    store.migrate().await?;
    Ok(store)
}
