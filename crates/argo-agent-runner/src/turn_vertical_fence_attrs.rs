//! cu1 (DEC-050 G1) + DEC-049 worker runtime-attribute readers, split from
//! `turn_vertical_fence.rs` to keep it under the 500-LOC ceiling. CHILD module
//! of the fence, so `super::*` brings the parent's imports.
use super::*;

/// cu1 (DEC-050 G1): read the worker's scoped filesystem sandbox root from
/// the `worker.fs_root` column. Returns `None` when the column is NULL/empty,
/// the row is absent, or the read errors — the fence then emits no `Fs*` cap
/// (default-deny preserved). A read error is logged, not fatal: a worker
/// without a resolvable sandbox simply cannot touch the filesystem, which is
/// the safe failure mode. Mirrors the direct-scalar read pattern the
/// filesystem executor uses (`worker_actions_executor_stubs_filesystem`),
/// keyed on the SERVER-AUTHORITATIVE `tenant_id` + `worker_id`.
pub(crate) async fn read_worker_fs_root(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: argo_store::WorkerId,
) -> Option<String> {
    let row: Result<Option<Option<String>>, sqlx::Error> =
        sqlx::query_scalar("SELECT fs_root FROM worker WHERE id = ? AND tenant_id = ?")
            .bind(worker_id.to_string())
            .bind(tenant_id.to_string())
            .fetch_optional(store.pool())
            .await;
    match row {
        Ok(found) => found
            .flatten()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        Err(e) => {
            tracing::warn!(
                target: "agent_runner::fence",
                error = %e,
                worker_id = %worker_id.0,
                "could not read worker.fs_root for the filesystem capability fence; \
                 treating as no-sandbox (default-deny, no Fs cap granted)"
            );
            None
        }
    }
}

/// DEC-049 master switch for worker child-teammate spawning. Reads
/// `ARGO_WORKER_SWARM`; default OFF. Accepts `1`/`true`/`yes`/`on`
/// (case-insensitive) as on, everything else (including unset) as off —
/// mirrors the `ARGO_DUTY_AUTOFIRE` posture. Operator-only flip after a
/// blast-radius review.
#[must_use]
pub(crate) fn swarm_enabled() -> bool {
    matches!(
        std::env::var("ARGO_WORKER_SWARM")
            .ok()
            .as_deref()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}
