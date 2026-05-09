//! Session, audit, judge, forge, and routing-decision read+write
//! impls — extracted from `lib.rs` to keep that file under the
//! 500-LOC ceiling (Stream `claude-argo-store-lib-split`). No
//! behaviour change vs. the historical inline implementations;
//! every method retains its public signature.
use crate::models::{
    AuditRow, ForgeRunRow, JudgeRow, RoutingDecisionRow, SessionHeader, SessionHeaderRow,
};
use crate::{SessionStore, StoreResult, WorkerId};
use argo_types::{ArgoError, RoutingDecision, SessionId, TenantId, WorkspaceId};
use chrono::Utc;

/// Session creation fields, including optional worker ownership.
pub struct SessionCreate<'a> {
    /// Tenant that owns the session.
    pub tenant_id: TenantId,
    /// Workspace where the session starts.
    pub workspace_id: WorkspaceId,
    /// Model id to store on the session.
    pub model: &'a str,
    /// Provider id to store on the session.
    pub provider: &'a str,
    /// Optional human-readable title.
    pub title: Option<&'a str>,
    /// Optional vertical template binding.
    pub vertical_id: Option<&'a str>,
    /// Optional worker identity that started or owns the session.
    pub worker_id: Option<WorkerId>,
}

impl SessionStore {
    /// Create a new session under a tenant + workspace.
    /// `vertical_id` binds the session to a vertical template
    /// (#651); pass `None` for ad-hoc / legacy sessions.
    pub async fn create_session(
        &self,
        tenant_id: TenantId,
        workspace_id: WorkspaceId,
        model: &str,
        provider: &str,
        title: Option<&str>,
        vertical_id: Option<&str>,
    ) -> StoreResult<SessionId> {
        self.create_session_with_worker(SessionCreate {
            tenant_id,
            workspace_id,
            model,
            provider,
            title,
            vertical_id,
            worker_id: None,
        })
        .await
    }

    /// Create a new session and optionally stamp its owning worker.
    pub async fn create_session_with_worker(&self, input: SessionCreate<'_>) -> StoreResult<SessionId> {
        let id = SessionId::new();
        let started_at = Utc::now().timestamp_micros();
        let worker_blob = input.worker_id.map(|w| w.0.as_bytes().to_vec());
        sqlx::query(
            "INSERT INTO session (id, tenant_id, workspace_id, model, provider, title, started_at, vertical_id, worker_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.0.as_bytes().as_slice())
        .bind(input.tenant_id.0.as_bytes().as_slice())
        .bind(input.workspace_id.0.as_bytes().as_slice())
        .bind(input.model)
        .bind(input.provider)
        .bind(input.title)
        .bind(started_at)
        .bind(input.vertical_id)
        .bind(worker_blob.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("create_session: {e}")))?;
        if let Some(worker) = input.worker_id {
            crate::worker_presence::touch_presence(&self.pool, &worker.to_string()).await?;
        }
        Ok(id)
    }

    /// Fetch a session header.
    pub async fn get_session(&self, session_id: SessionId) -> StoreResult<Option<SessionHeader>> {
        let row = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id
             FROM session WHERE id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("get_session: {e}")))?;

        row.map(SessionHeader::try_from).transpose()
    }

    /// List sessions across all tenants, ordered by recency. Used
    /// by the CLI's `argo session list` when no tenant filter is
    /// provided.
    pub async fn list_all_sessions(&self, limit: u32) -> StoreResult<Vec<SessionHeader>> {
        let rows = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id
             FROM session ORDER BY started_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_all_sessions: {e}")))?;
        rows.into_iter().map(SessionHeader::try_from).collect()
    }

    /// List sessions for a tenant, ordered by recency.
    pub async fn list_sessions(
        &self,
        tenant_id: TenantId,
        limit: u32,
    ) -> StoreResult<Vec<SessionHeader>> {
        let rows = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id
             FROM session WHERE tenant_id = ?
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_sessions: {e}")))?;

        rows.into_iter().map(SessionHeader::try_from).collect()
    }

    /// Delete every session whose `started_at` is older than
    /// `cutoff_micros`. Cascades through `message`, `tool_call`,
    /// `audit_record`, `routing_decision`, and `judge_record` via
    /// SQLite ON DELETE CASCADE. Returns the number of session
    /// rows deleted.
    pub async fn prune_sessions_before(&self, cutoff_micros: i64) -> StoreResult<u64> {
        let result = sqlx::query("DELETE FROM session WHERE started_at < ?")
            .bind(cutoff_micros)
            .execute(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("prune_sessions_before: {e}")))?;
        Ok(result.rows_affected())
    }

    /// Count sessions whose `started_at` is older than
    /// `cutoff_micros`. Used by `argo db prune --dry-run`.
    pub async fn count_sessions_before(&self, cutoff_micros: i64) -> StoreResult<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE started_at < ?")
            .bind(cutoff_micros)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("count_sessions_before: {e}")))
    }

    /// List judge records for a session, oldest first. Used by
    /// `argo judge inspect <session_id>`.
    pub async fn list_judge_records(&self, session_id: SessionId) -> StoreResult<Vec<JudgeRow>> {
        let rows = sqlx::query_as::<_, JudgeRow>(
            "SELECT subject_kind, subject_ref, verdict, reason, judge_provider, \
             judge_model, rubric_id, rubric_version, created_at \
             FROM judge_record WHERE session_id = ? \
             ORDER BY created_at ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_judge_records: {e}")))?;
        Ok(rows)
    }

    /// Stream a session's audit-log rows in `seq` order. Used by
    /// `argo trace --session <id>` to render the per-session
    /// trace timeline.
    pub async fn list_audit_for_session(
        &self,
        session_id: SessionId,
    ) -> StoreResult<Vec<AuditRow>> {
        let rows = sqlx::query_as::<_, AuditRow>(
            "SELECT seq, kind, payload_json, created_at FROM audit_record \
             WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_audit_for_session: {e}")))?;
        Ok(rows)
    }

    /// List recent Forge runs (most-recent first). Used by
    /// `argo forge runs list` to surface optimization history.
    pub async fn list_forge_runs(&self, limit: u32) -> StoreResult<Vec<ForgeRunRow>> {
        let rows = sqlx::query_as::<_, ForgeRunRow>(
            "SELECT id, skill_name, started_at, ended_at, base_version, \
             candidate_version, iterations, spend_usd_micros, \
             final_score, status FROM forge_run \
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_forge_runs: {e}")))?;
        Ok(rows)
    }

    /// Persist a [`RoutingDecision`] (Amendment 1, §24.4 acceptance: "all
    /// non-Fixed decisions persisted in `routing_decision`"). The cost
    /// router calls this for every turn when policy ≠ Fixed.
    pub async fn persist_routing_decision(&self, d: &RoutingDecision) -> StoreResult<()> {
        let decided_at = d.classified_at.timestamp_micros();
        sqlx::query(
            "INSERT INTO routing_decision (id, session_id, turn, policy, tier, \
             classifier_latency_ms, chosen_provider, chosen_model, reason, decided_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(d.id.0.as_bytes().as_slice())
        .bind(d.session_id.0.as_bytes().as_slice())
        .bind(i64::from(d.turn))
        .bind(&d.policy)
        .bind(&d.tier)
        .bind(i64::try_from(d.classifier_latency_ms).unwrap_or(i64::MAX))
        .bind(&d.chosen_provider_model.provider)
        .bind(&d.chosen_provider_model.model)
        .bind(&d.reason)
        .bind(decided_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("persist_routing_decision: {e}")))?;
        Ok(())
    }

    /// List routing decisions for `session_id` in turn order.
    /// Used by `argo route inspect` to surface the cost-router's
    /// per-turn picks (Amendment 1, §24.4).
    pub async fn list_routing_decisions(
        &self,
        session_id: SessionId,
    ) -> StoreResult<Vec<RoutingDecision>> {
        let rows: Vec<RoutingDecisionRow> = sqlx::query_as::<_, RoutingDecisionRow>(
            "SELECT id, session_id, turn, policy, tier, classifier_latency_ms, \
             chosen_provider, chosen_model, reason, decided_at \
             FROM routing_decision WHERE session_id = ? \
             ORDER BY turn ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_routing_decisions: {e}")))?;
        Ok(rows.into_iter().map(RoutingDecision::from).collect())
    }
}

#[cfg(test)]
#[path = "session_worker_id_tests.rs"]
mod session_worker_id_tests;
