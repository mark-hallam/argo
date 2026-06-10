//! DEC-049 Stage 2 handoff helpers, split from `persist_handoffs.rs` to keep
//! that file under the 500-LOC ceiling. A CHILD module of `persist_handoffs`,
//! so it can reach the parent's private row helpers via `super::*`.
use super::*;

impl OrchestratorStore {
    /// Insert a handoff but ATOMICALLY refuse if the session already holds
    /// `max_active` in-flight (`Pending`/`Acknowledged`) handoffs. The active
    /// count and the insert run in ONE `BEGIN IMMEDIATE` transaction, so two
    /// concurrent callers cannot both pass the check and overshoot the cap —
    /// closing the TOCTOU a plain count-then-insert has (DEC-049 fan-out cap,
    /// Stage 2). Returns `Ok(true)` when inserted, `Ok(false)` when refused
    /// because the cap is already reached.
    pub async fn try_insert_handoff_capped(
        &self,
        tenant_id: TenantId,
        handoff: &Handoff,
        max_active: usize,
    ) -> ArgoResult<bool> {
        self.ensure_schema().await?;
        // Same tenant gate as `insert_handoff`: the handoff's session must
        // belong to the caller's tenant (else audit + refuse).
        if self
            .find_session(tenant_id, &handoff.session_id)
            .await?
            .is_none()
        {
            try_audit_cross_tenant_handoff_attempt(self.pool(), tenant_id, handoff).await?;
            return Err(ArgoError::Schema("handoff session not found".into()));
        }
        let mut tx = self
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| ArgoError::Provider(format!("handoff cap tx begin: {e}")))?;
        let active: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM handoff WHERE session_id = ? AND status IN (?, ?)",
        )
        .bind(session_id_blob(&handoff.session_id)?.as_slice())
        .bind(handoff_status_label(HandoffStatus::Pending))
        .bind(handoff_status_label(HandoffStatus::Acknowledged))
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("handoff active count: {e}")))?;
        if active as usize >= max_active {
            // Drop the tx without inserting — the cap is reached.
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO handoff (id, session_id, from_worker_id, \"to\", expected_outcome, \
             payload, status, created_at, deadline) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(handoff.id.as_str())
        .bind(session_id_blob(&handoff.session_id)?.as_slice())
        .bind(handoff.from.to_string())
        .bind(step_json(&handoff.to)?)
        .bind(&handoff.expected_outcome)
        .bind(to_json(&handoff.payload, "handoff.payload")?)
        .bind(handoff_status_label(handoff.status))
        .bind(handoff.created_at.timestamp_micros())
        .bind(handoff.deadline.map(|d| d.timestamp_micros()))
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert capped handoff: {e}")))?;
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("handoff cap tx commit: {e}")))?;
        Ok(true)
    }

    /// Tenant that owns `worker_id` per the `worker` table, or `None` when the
    /// worker row does not exist (a not-yet-materialized target is allowed; only
    /// a target that EXISTS under a DIFFERENT tenant is a containment breach the
    /// caller must refuse — DEC-049 Stage 2 tenant target validation). Returns
    /// `None` (not an error) when the `worker` table is absent (minimal DBs).
    pub async fn worker_tenant(&self, worker_id: WorkerId) -> ArgoResult<Option<TenantId>> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'worker'",
        )
        .fetch_one(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("worker table probe: {e}")))?;
        if exists == 0 {
            return Ok(None);
        }
        let row: Option<String> = sqlx::query_scalar("SELECT tenant_id FROM worker WHERE id = ?")
            .bind(worker_id.0.to_string())
            .fetch_optional(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker tenant lookup: {e}")))?;
        match row {
            None => Ok(None),
            Some(t) => Uuid::parse_str(&t)
                .map(|u| Some(TenantId(u)))
                .map_err(|e| ArgoError::Schema(format!("worker.tenant_id parse: {e}"))),
        }
    }
}
