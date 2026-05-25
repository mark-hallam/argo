//! Session, audit, judge, forge, and routing-decision read+write
//! impls — extracted from `lib.rs` to keep that file under the
//! 500-LOC ceiling (Stream `claude-argo-store-lib-split`). No
//! behaviour change vs. the historical inline implementations;
//! every method retains its public signature.
use crate::models::{
    AuditRow, ForgeRunRow, JudgeRow, RoutingDecisionRow, SessionHeader, SessionHeaderRow,
};
use crate::{PromptCacheAffinity, SessionStore, StoreResult, WorkerId};
use argo_types::{ArgoError, RoutingDecision, SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use std::sync::Arc;

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
    /// Attach a [`PromptCacheAffinity`] registry to this store. When
    /// set, [`SessionStore::create_session_with_worker`] consults
    /// `acquire_or_mint` before minting a fresh `SessionId` so the
    /// same `(tenant_id, worker_id)` pair reuses its previous session
    /// id inside the registry's TTL window — exactly the condition
    /// Anthropic's prompt cache reads against. With no affinity
    /// attached the call site keeps its pre-brief
    /// `SessionId::new()`-per-call behavior.
    ///
    /// ```
    /// # async fn run() -> argo_store::StoreResult<()> {
    /// use argo_store::{SessionStore, PromptCacheAffinity};
    /// use std::{sync::Arc, time::Duration};
    /// let store = SessionStore::open(":memory:").await?
    ///     .with_affinity(Arc::new(PromptCacheAffinity::enabled(Duration::from_secs(270))));
    /// store.migrate().await?;
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn with_affinity(mut self, affinity: Arc<PromptCacheAffinity>) -> Self {
        self.prompt_cache_affinity = Some(affinity);
        self
    }

    /// Read-only view of the attached [`PromptCacheAffinity`]
    /// registry, if one was wired via [`SessionStore::with_affinity`].
    /// Used by `argo-agent-runner::turn` to record observed
    /// `(tenant, worker, session_id)` triples once the session row
    /// loads.
    #[must_use]
    pub fn prompt_cache_affinity(&self) -> Option<&Arc<PromptCacheAffinity>> {
        self.prompt_cache_affinity.as_ref()
    }

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
    ///
    /// When the store has an attached
    /// [`crate::PromptCacheAffinity`] registry (see
    /// [`crate::SessionStore::with_affinity`]) **and** `input.worker_id`
    /// is `Some`, the registry's
    /// `acquire_or_mint(tenant, worker)` is consulted before
    /// minting. Within the TTL window the call resolves to the
    /// previously-recorded id — the underlying `INSERT` uses
    /// `ON CONFLICT(id) DO NOTHING` so the existing session row is
    /// reused. With no affinity attached (or a `None`
    /// `worker_id`) each call mints a fresh `SessionId::new()` and
    /// inserts a new row, matching the pre-brief behavior.
    pub async fn create_session_with_worker(
        &self,
        input: SessionCreate<'_>,
    ) -> StoreResult<SessionId> {
        let id = match (&self.prompt_cache_affinity, input.worker_id) {
            (Some(affinity), Some(worker)) => {
                affinity.acquire_or_mint(input.tenant_id, &worker.to_string())
            }
            _ => SessionId::new(),
        };
        let started_at = Utc::now().timestamp_micros();
        let worker_blob = input.worker_id.map(|w| w.0.as_bytes().to_vec());
        sqlx::query(
            "INSERT INTO session (id, tenant_id, workspace_id, model, provider, title, started_at, vertical_id, worker_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING",
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

    /// Fetch a session header by id, without tenant scoping.
    ///
    /// Used by internal callers (worker turn helpers, replay
    /// tooling) that have already established tenant context out
    /// of band. **Gateway handlers must use
    /// [`SessionStore::get_session_for_tenant`] instead** — they
    /// derive the caller's tenant from the `X-Argo-Tenant-Id`
    /// header and need the SQL-level filter to refuse
    /// cross-tenant reads. See sweep report
    /// `docs/skycot/evals/scheduled/2026-05-23/eval-gateway-uuid-
    /// bound-routes-tenant-ownership-sweep.md` §3 cluster 2 for
    /// the leak this gap previously enabled.
    pub async fn get_session(&self, session_id: SessionId) -> StoreResult<Option<SessionHeader>> {
        let row = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id, routing_policy
             FROM session WHERE id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("get_session: {e}")))?;

        row.map(SessionHeader::try_from).transpose()
    }

    /// Tenant-scoped sibling of [`SessionStore::get_session`].
    /// Returns `Ok(None)` when the row doesn't exist OR belongs
    /// to a different tenant, so gateway handlers can map the
    /// result straight to the "existence-hide" 404 shape used
    /// across the audit-replay / audit-detail family.
    ///
    /// `tenant_id_str` is the raw `X-Argo-Tenant-Id` header
    /// value — typically the UUID string form of [`TenantId`].
    /// A non-UUID value matches no row and resolves to
    /// `Ok(None)` (the gateway hides existence either way).
    pub async fn get_session_for_tenant(
        &self,
        session_id: SessionId,
        tenant_id_str: &str,
    ) -> StoreResult<Option<SessionHeader>> {
        let Ok(tenant_uuid) = uuid::Uuid::parse_str(tenant_id_str) else {
            return Ok(None);
        };
        let row = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id, routing_policy
             FROM session WHERE id = ? AND tenant_id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .bind(tenant_uuid.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("get_session_for_tenant: {e}")))?;

        row.map(SessionHeader::try_from).transpose()
    }

    /// Lightweight existence + ownership check. Returns `true`
    /// only when the session row exists AND its `tenant_id`
    /// matches `tenant_id_str` parsed as a UUID. Used as the
    /// pre-flight gate for per-session aggregate handlers
    /// (audit rollups, stats, turns) whose existing SQL is
    /// session-id scoped only.
    pub async fn session_belongs_to_tenant(
        &self,
        session_id: SessionId,
        tenant_id_str: &str,
    ) -> StoreResult<bool> {
        let Ok(tenant_uuid) = uuid::Uuid::parse_str(tenant_id_str) else {
            return Ok(false);
        };
        let exists: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM session WHERE id = ? AND tenant_id = ? LIMIT 1")
                .bind(session_id.0.as_bytes().as_slice())
                .bind(tenant_uuid.as_bytes().as_slice())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("session_belongs_to_tenant: {e}")))?;
        Ok(exists.is_some())
    }

    /// List sessions across all tenants, ordered by recency. Used
    /// by the CLI's `argo session list` when no tenant filter is
    /// provided.
    pub async fn list_all_sessions(&self, limit: u32) -> StoreResult<Vec<SessionHeader>> {
        let rows = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id, routing_policy
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
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id, routing_policy
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
        if !self
            .routing_session_matches_tenant(d.session_id, d.tenant_id)
            .await?
        {
            return Err(ArgoError::NotFound(format!(
                "routing session {} for tenant {}",
                d.session_id, d.tenant_id
            )));
        }
        let decided_at = d.classified_at.timestamp_micros();
        sqlx::query(
            "INSERT INTO routing_decision (id, tenant_id, session_id, turn, policy, tier, \
             classifier_latency_ms, chosen_provider, chosen_model, reason, decided_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(d.id.0.as_bytes().as_slice())
        .bind(d.tenant_id.0.as_bytes().as_slice())
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

    async fn routing_session_matches_tenant(
        &self,
        session_id: SessionId,
        tenant_id: TenantId,
    ) -> StoreResult<bool> {
        let row: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM session WHERE id = ? AND tenant_id = ? LIMIT 1")
                .bind(session_id.0.as_bytes().as_slice())
                .bind(tenant_id.0.as_bytes().as_slice())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("routing session tenant check: {e}")))?;
        Ok(row.is_some())
    }

    /// List routing decisions for `session_id` in turn order.
    /// Used by `argo route inspect` to surface the cost-router's
    /// per-turn picks (Amendment 1, §24.4).
    pub async fn list_routing_decisions(
        &self,
        session_id: SessionId,
    ) -> StoreResult<Vec<RoutingDecision>> {
        let rows: Vec<RoutingDecisionRow> = sqlx::query_as::<_, RoutingDecisionRow>(
            "SELECT id, tenant_id, session_id, turn, policy, tier, classifier_latency_ms, \
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

    /// List routing decisions for one tenant in most-recent-first order.
    pub async fn list_routing_decisions_for_tenant(
        &self,
        tenant_id: TenantId,
    ) -> StoreResult<Vec<RoutingDecision>> {
        let rows: Vec<RoutingDecisionRow> = sqlx::query_as::<_, RoutingDecisionRow>(
            "SELECT id, tenant_id, session_id, turn, policy, tier, classifier_latency_ms, \
             chosen_provider, chosen_model, reason, decided_at \
             FROM routing_decision WHERE tenant_id = ? \
             ORDER BY decided_at DESC, turn ASC",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_routing_decisions_for_tenant: {e}")))?;
        Ok(rows.into_iter().map(RoutingDecision::from).collect())
    }
}

#[cfg(test)]
#[path = "session_worker_id_tests.rs"]
mod session_worker_id_tests;

#[cfg(test)]
#[path = "session_query_affinity_tests.rs"]
mod session_query_affinity_tests;
