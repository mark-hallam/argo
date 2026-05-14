//! Operator resume for worker circuit breaker.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{circuit, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::str::FromStr;

pub(crate) async fn run(
    worker_id: &str,
    tenant: &str,
    reason: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    uuid::Uuid::parse_str(tenant)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{tenant}': {e}")))?;
    let worker = WorkerId::from_str(worker_id)
        .map_err(|e| ArgoError::Schema(format!("invalid worker id '{worker_id}': {e}")))?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    let worker_id = worker.to_string();
    let state = circuit::resume(store.pool(), &worker_id, reason).await?;
    if json {
        let body = serde_json::json!({
            "worker_id": worker_id,
            "tenant": tenant,
            "circuit_state": state,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("{} → {}", worker_id, state.as_str());
    }
    Ok(())
}
