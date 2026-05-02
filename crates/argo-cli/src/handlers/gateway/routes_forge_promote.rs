//! Forge run write-side helpers — `/forge/runs/<id>/promote`
//! and `/forge/runs/abort-all`. Lifted out of
//! `routes_post_writes.rs` so that file stays under the
//! 500-LOC ceiling.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `POST /forge/runs/abort-all` — flip every forge_run row
/// with `status='running'` to `'aborted'`. Returns 200 +
/// `{aborted:N}`. Useful for emergency stop / fresh-slate
/// before a workspace migration.
pub(super) async fn forge_abort_all_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let rows = match store.list_forge_runs(500).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"list_forge_runs: {e}\"}}"),
            );
        }
    };
    let mut aborted: u64 = 0;
    for r in rows.into_iter().filter(|r| r.status == "running") {
        let id = match uuid::Uuid::from_slice(&r.id) {
            Ok(u) => u,
            Err(_) => continue,
        };
        if let Ok(n) = store.abort_forge_run(id).await {
            aborted += n;
        }
    }
    (
        "200 OK",
        format!("{{\"aborted\":{aborted}}}"),
    )
}

/// `POST /forge/runs/sweep[?older-than-minutes=N]` — flip
/// stalled `running` rows to `failed` (recovery for a gateway
/// crash mid-optimize). Default cutoff is 60m. Returns
/// `{swept:N, older_than_minutes:M}`.
pub(super) async fn forge_sweep_response(
    older_than_minutes: i64,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let cutoff_micros = (chrono::Utc::now() - chrono::Duration::minutes(older_than_minutes))
        .timestamp_micros();
    match store.sweep_stalled_forge_runs(cutoff_micros).await {
        Ok(n) => (
            "200 OK",
            format!("{{\"swept\":{n},\"older_than_minutes\":{older_than_minutes}}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"sweep_stalled_forge_runs: {e}\"}}"),
        ),
    }
}

/// Convenience wrapper over `POST /skills/<name>/promote` —
/// the gateway looks up `(skill_name, candidate_version)`
/// from the forge_run row. 200 + `{promoted, name, version,
/// skill_id, run_id}`. 400 on bad uuid; 404 when no row
/// matches; 409 when the run hasn't produced a candidate
/// yet.
pub(super) async fn forge_promote_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid forge run id '{raw_id}'\"}}"),
            );
        }
    };
    let row = match store.get_forge_run(id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no forge run with id {raw_id}\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"get_forge_run: {e}\"}}"),
            );
        }
    };
    let Some(version) = row.candidate_version.clone() else {
        return (
            "409 Conflict",
            format!(
                "{{\"error\":\"forge run {raw_id} has no candidate_version yet — \
                 wait for at least one iteration to complete\"}}"
            ),
        );
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.promote(&row.skill_name, &version).await {
        Ok(s) => {
            let resp = serde_json::json!({
                "promoted": true,
                "name": row.skill_name,
                "version": version,
                "skill_id": s.id.0.to_string(),
                "run_id": raw_id,
            });
            (
                "200 OK",
                serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"promote: {e}\"}}"),
        ),
    }
}

/// `POST /forge/runs/<id>/requeue` — copy a finished/failed/
/// aborted run's queued payload into a fresh forge_run row
/// with status='queued'. The daemon picks it up on its next
/// tick. The source row keeps its terminal status as a
/// historical record. Returns 201 + `{requeued, source_id,
/// run_id}`. 404 when no row matches OR the source has no
/// forge_queue payload (e.g. ran via ?in_process=true).
/// 400 on a malformed id.
pub(super) async fn forge_requeue_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid forge run id '{raw_id}'\"}}"),
            );
        }
    };
    match store.requeue_forge_run(id).await {
        Ok(Some(new_id)) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "requeued": true,
                "source_id": id.to_string(),
                "run_id": new_id.to_string(),
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Ok(None) => (
            "404 Not Found",
            format!(
                "{{\"error\":\"no forge_queue payload for run {raw_id} (was it created with ?in_process=true?)\"}}"
            ),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"requeue_forge_run: {e}\"}}"),
        ),
    }
}
