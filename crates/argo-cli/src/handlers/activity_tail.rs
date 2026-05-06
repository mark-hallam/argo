//! `argo activity tail` — live worker activity feed.

use crate::cli::ActivityCmd;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::time::Duration;

const TAIL_BACKFILL_LIMIT: u32 = 500;
const TAIL_BATCH_LIMIT: u32 = 100;

pub async fn run(cmd: ActivityCmd) -> ArgoResult<()> {
    match cmd {
        ActivityCmd::Tail {
            tenant,
            worker,
            since,
            json,
        } => {
            let store = super::activity_since::open_store().await?;
            let query = super::activity_since::tail_query(
                since.as_deref(),
                tenant,
                worker,
                TAIL_BACKFILL_LIMIT,
            )?;
            tail_loop(&store, query, json, None).await
        }
        ActivityCmd::Since {
            duration,
            limit,
            tenant,
            worker,
            json,
        } => super::activity_since::run(&duration, limit, tenant, worker, json).await,
    }
}

async fn tail_loop(
    store: &SessionStore,
    query: super::activity_since::ActivityQuery,
    json: bool,
    max_events: Option<usize>,
) -> ArgoResult<()> {
    let mut emitted = 0usize;
    let mut last_seq = if query.since_micros.is_some() {
        let rows = super::activity_since::query_activity(store, &query).await?;
        emitted += print_rows(&rows, json)?;
        rows.iter()
            .map(|row| row.seq)
            .max()
            .unwrap_or(super::activity_since::max_seq(store).await?)
    } else {
        super::activity_since::max_seq(store).await?
    };
    if max_events.is_some_and(|max| emitted >= max) {
        return Ok(());
    }

    let mut retried = false;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => return Ok(()),
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
        let next_query = super::activity_since::after_query(&query, last_seq, TAIL_BATCH_LIMIT);
        match super::activity_since::query_activity(store, &next_query).await {
            Ok(rows) => {
                retried = false;
                if let Some(max) = rows.iter().map(|row| row.seq).max() {
                    last_seq = last_seq.max(max);
                }
                emitted += print_rows(&rows, json)?;
                if max_events.is_some_and(|max| emitted >= max) {
                    return Ok(());
                }
            }
            Err(err) if !retried => {
                retried = true;
                eprintln!("activity stream read failed: {err}; retrying once");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(err) => {
                return Err(ArgoError::Provider(format!(
                    "activity stream disconnected after retry: {err}"
                )));
            }
        }
    }
}

fn print_rows(rows: &[super::activity_since::ActivityRow], json: bool) -> ArgoResult<usize> {
    if json {
        super::activity_since::print_ndjson(rows)?;
    } else {
        for row in rows {
            println!("{}", super::activity_since::format_text_row(row));
        }
    }
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_audit::{AuditKind, AuditSink};
    use argo_store::SessionStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn tail_loop_reads_backfill_then_stops_at_max_events() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, workspace) = store
            .ensure_workspace("activity-tail", "/tmp/activity-tail")
            .await
            .unwrap();
        let session = store
            .create_session(tenant, workspace, "mock", "mock", Some("tail"), None)
            .await
            .unwrap();
        let sink = AuditSink::new(Arc::clone(&store));
        sink.write(
            Some(session),
            AuditKind::ToolCall,
            serde_json::json!({"worker_id":"worker-tail","tool":"shell"}),
        )
        .await
        .unwrap();

        let query = super::super::activity_since::tail_query(
            Some("1d"),
            Some(tenant.to_string()),
            Some("worker-tail".into()),
            10,
        )
        .unwrap();
        tail_loop(&store, query, true, Some(1)).await.unwrap();
    }
}
