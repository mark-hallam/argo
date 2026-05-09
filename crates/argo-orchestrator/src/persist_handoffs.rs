//! Handoff-row persistence for the orchestrator session schema.
//!
//! Extracted from `persist.rs` (HARD RULE 6 defensive split — see
//! brief `claude-orchestrator-persist-split`). The
//! [`OrchestratorStore`] inherent-impl block in this file extends
//! the one in `persist.rs`; SQL row codecs and the cancel-audit
//! chain hand-off live alongside the methods that need them. Pure
//! refactor: no public-API or behaviour changes.
use crate::persist::{
    from_json, micros_to_utc, opt_micros, session_id_blob, session_id_from_blob, tenant_blob,
    to_json, ORCHESTRATOR_PROVIDER,
};
use crate::{EscalationStep, Handoff, HandoffId, HandoffStatus, OrchestratorStore, SessionId};
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use std::str::FromStr;
use uuid::Uuid;

const HANDOFF_SELECT_BY_ID: &str = "SELECT h.id, h.session_id, h.from_worker_id, h.\"to\", \
h.expected_outcome, h.payload, h.status, h.created_at, h.deadline FROM handoff h \
JOIN session s ON s.id = h.session_id WHERE h.id = ? AND s.tenant_id = ? AND s.provider = ?";

#[derive(sqlx::FromRow)]
struct RawHandoff { id: String, session_id: Vec<u8>, from_worker_id: String, to: String, expected_outcome: String, payload: String, status: String, created_at: i64, deadline: Option<i64> }

impl TryFrom<RawHandoff> for Handoff {
    type Error = ArgoError;
    fn try_from(r: RawHandoff) -> ArgoResult<Self> {
        Ok(Self { id: HandoffId::from_string(r.id), session_id: session_id_from_blob(&r.session_id)?,
            from: WorkerId::from_str(&r.from_worker_id).map_err(|e| ArgoError::Schema(format!("handoff.from_worker_id: {e}")))?,
            to: step_from_json(&r.to)?, expected_outcome: r.expected_outcome,
            payload: from_json(&r.payload, "handoff.payload")?, status: parse_handoff_status(&r.status)?,
            created_at: micros_to_utc(r.created_at, "handoff.created_at")?,
            deadline: opt_micros(r.deadline, "handoff.deadline")? })
    }
}

fn handoff_status_label(s: HandoffStatus) -> &'static str { match s { HandoffStatus::Pending => "pending", HandoffStatus::Acknowledged => "acknowledged", HandoffStatus::Settled => "settled", HandoffStatus::Expired => "expired", HandoffStatus::Cancelled => "cancelled" } }
fn parse_handoff_status(s: &str) -> ArgoResult<HandoffStatus> { match s { "pending" => Ok(HandoffStatus::Pending), "acknowledged" => Ok(HandoffStatus::Acknowledged), "settled" => Ok(HandoffStatus::Settled), "expired" => Ok(HandoffStatus::Expired), "cancelled" => Ok(HandoffStatus::Cancelled), other => Err(ArgoError::Schema(format!("handoff.status: {other}"))) } }
fn handoff_transition_allowed(current: HandoffStatus, next: HandoffStatus) -> bool { matches!((current, next), (HandoffStatus::Pending, _) | (HandoffStatus::Acknowledged, HandoffStatus::Acknowledged | HandoffStatus::Settled | HandoffStatus::Cancelled) | (HandoffStatus::Settled, HandoffStatus::Settled) | (HandoffStatus::Expired, HandoffStatus::Expired) | (HandoffStatus::Cancelled, HandoffStatus::Cancelled)) }
fn is_terminal_handoff_status(status: HandoffStatus) -> bool { matches!(status, HandoffStatus::Settled | HandoffStatus::Expired | HandoffStatus::Cancelled) }
fn step_json(s: &EscalationStep) -> ArgoResult<String> { match s { EscalationStep::Worker(id) => to_json(&serde_json::json!({"kind":"worker","worker_id":id.to_string()}), "handoff.to"), EscalationStep::Human(addr) => to_json(&serde_json::json!({"kind":"human","addr":addr}), "handoff.to") } }
fn step_from_json(json: &str) -> ArgoResult<EscalationStep> {
    let v: serde_json::Value = from_json(json, "handoff.to")?;
    match v.get("kind").and_then(|v| v.as_str()) {
        Some("worker") => { let raw = v.get("worker_id").and_then(|v| v.as_str()).ok_or_else(|| ArgoError::Schema("handoff.to worker_id missing".into()))?; WorkerId::from_str(raw).map(EscalationStep::Worker).map_err(|e| ArgoError::Schema(format!("handoff.to worker_id: {e}"))) }
        Some("human") => v.get("addr").and_then(|v| v.as_str()).map(|s| EscalationStep::Human(s.to_string())).ok_or_else(|| ArgoError::Schema("handoff.to addr missing".into())),
        _ => Err(ArgoError::Schema("handoff.to kind unknown".into())),
    }
}

async fn write_handoff_cancelled_audit(pool: &sqlx::Pool<sqlx::Sqlite>, h: &Handoff, note: Option<&str>) -> ArgoResult<()> {
    let kind = "handoff_cancelled"; let payload = serde_json::json!({"action":"cancel","handoff_id":h.id.as_str(),"session_id":h.session_id.as_str(),"status":"cancelled","note":note}); let canonical = serde_json::to_string(&payload).map_err(|e| ArgoError::Schema(format!("audit payload: {e}")))?; let sid = Uuid::parse_str(h.session_id.as_str()).map_err(|e| ArgoError::Schema(format!("handoff.session_id: {e}")))?; let parent = last_session_hash(pool, &sid).await?; let hash = argo_audit::compute_chain_hash(&parent, kind, &canonical); let id = Uuid::now_v7();
    sqlx::query("INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)").bind(id.as_bytes().as_slice()).bind(sid.as_bytes().as_slice()).bind(kind).bind(canonical).bind(parent.as_slice()).bind(hash.as_slice()).bind(Utc::now().timestamp_micros()).execute(pool).await.map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?; Ok(())
}
async fn last_session_hash(pool: &sqlx::Pool<sqlx::Sqlite>, sid: &Uuid) -> ArgoResult<[u8; 32]> { let row: Option<Vec<u8>> = sqlx::query_scalar("SELECT self_hash FROM audit_record WHERE session_id = ? ORDER BY seq DESC LIMIT 1").bind(sid.as_bytes().as_slice()).fetch_optional(pool).await.map_err(|e| ArgoError::Provider(format!("audit last_hash: {e}")))?; let mut buf = [0u8; 32]; if let Some(bytes) = row { if bytes.len() == 32 { buf.copy_from_slice(&bytes); } } Ok(buf) }

impl OrchestratorStore {
    /// Insert a handoff for an existing tenant-scoped session.
    pub async fn insert_handoff(&self, tenant_id: TenantId, handoff: &Handoff) -> ArgoResult<()> {
        self.ensure_schema().await?;
        if self
            .find_session(tenant_id, &handoff.session_id)
            .await?
            .is_none()
        {
            return Err(ArgoError::Schema("handoff session not found".into()));
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
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert handoff: {e}")))?;
        Ok(())
    }

    /// Find one handoff inside a tenant.
    pub async fn find_handoff(
        &self,
        tenant_id: TenantId,
        handoff_id: &HandoffId,
    ) -> ArgoResult<Option<Handoff>> {
        self.ensure_schema().await?;
        let row = sqlx::query_as::<_, RawHandoff>(HANDOFF_SELECT_BY_ID)
            .bind(handoff_id.as_str())
            .bind(tenant_blob(tenant_id).as_slice())
            .bind(ORCHESTRATOR_PROVIDER)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("find handoff: {e}")))?;
        row.map(Handoff::try_from).transpose()
    }

    /// List handoffs attached to a tenant-scoped session.
    pub async fn list_handoffs_for_session(
        &self,
        tenant_id: TenantId,
        session_id: &SessionId,
    ) -> ArgoResult<Vec<Handoff>> {
        self.ensure_schema().await?;
        let rows = sqlx::query_as::<_, RawHandoff>(
            "SELECT h.id, h.session_id, h.from_worker_id, h.\"to\", h.expected_outcome, \
             h.payload, h.status, h.created_at, h.deadline FROM handoff h \
             JOIN session s ON s.id = h.session_id \
             WHERE h.session_id = ? AND s.tenant_id = ? AND s.provider = ? \
             ORDER BY h.created_at ASC, h.id ASC",
        )
        .bind(session_id_blob(session_id)?.as_slice())
        .bind(tenant_blob(tenant_id).as_slice())
        .bind(ORCHESTRATOR_PROVIDER)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list handoffs for session: {e}")))?;
        rows.into_iter().map(Handoff::try_from).collect()
    }

    /// List pending handoffs for one worker inside a tenant.
    pub async fn list_pending_handoffs_for_worker(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<Vec<Handoff>> {
        self.ensure_schema().await?;
        let rows = sqlx::query_as::<_, RawHandoff>(
            "SELECT h.id, h.session_id, h.from_worker_id, h.\"to\", h.expected_outcome, \
             h.payload, h.status, h.created_at, h.deadline FROM handoff h \
             JOIN session s ON s.id = h.session_id \
             WHERE s.tenant_id = ? AND s.provider = ? AND h.status = 'pending' \
             ORDER BY h.created_at ASC, h.id ASC",
        )
        .bind(tenant_blob(tenant_id).as_slice())
        .bind(ORCHESTRATOR_PROVIDER)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list pending handoffs: {e}")))?;
        rows.into_iter()
            .map(Handoff::try_from)
            .filter_map(|h| match h {
                Ok(h) if matches!(h.to, EscalationStep::Worker(id) if id == worker_id) => {
                    Some(Ok(h))
                }
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .collect()
    }

    /// Store the terminal-resolution timestamp for a handoff.
    pub async fn update_handoff_resolved_at(&self, handoff_id: &HandoffId, now_micros: i64) -> ArgoResult<()> {
        self.ensure_schema().await?; sqlx::query("UPDATE handoff SET resolved_at = ? WHERE id = ?").bind(now_micros).bind(handoff_id.as_str()).execute(self.pool()).await.map_err(|e| ArgoError::Provider(format!("update handoff resolved_at: {e}")))?; Ok(())
    }

    /// Update handoff status and write transition audit/timestamp side effects.
    pub async fn update_handoff_status(&self, tenant_id: TenantId, handoff_id: &HandoffId, status: HandoffStatus) -> ArgoResult<()> {
        let handoff = self.find_handoff(tenant_id, handoff_id).await?.ok_or_else(|| ArgoError::Schema("handoff not found".into()))?;
        let current = handoff.status;
        if !handoff_transition_allowed(current, status) { return Err(ArgoError::Invariant("invalid handoff status transition")); }
        sqlx::query("UPDATE handoff SET status = ? WHERE id = ?").bind(handoff_status_label(status)).bind(handoff_id.as_str()).execute(self.pool()).await.map_err(|e| ArgoError::Provider(format!("update handoff status: {e}")))?;
        if is_terminal_handoff_status(status) { self.update_handoff_resolved_at(handoff_id, Utc::now().timestamp_micros()).await?; }
        if matches!((current, status), (HandoffStatus::Pending, HandoffStatus::Acknowledged)) {
            crate::persist_audit::audit_worker_handoff(self.pool(), &handoff).await?;
        }
        Ok(())
    }

    /// Cancel a pending or acknowledged handoff and append its audit row.
    pub async fn cancel_handoff(&self, tenant_id: TenantId, handoff_id: &HandoffId, note: Option<&str>) -> ArgoResult<Handoff> {
        let mut handoff = self.find_handoff(tenant_id, handoff_id).await?.ok_or_else(|| ArgoError::Schema("handoff not found".into()))?; handoff.cancel()?;
        if let Some(note) = note.map(str::trim).filter(|n| !n.is_empty()) { let mut payload = handoff.payload.as_object().cloned().unwrap_or_default(); payload.insert("cancel_note".into(), serde_json::Value::String(note.to_string())); handoff.payload = serde_json::Value::Object(payload); }
        sqlx::query("UPDATE handoff SET status = ?, payload = ? WHERE id = ?").bind(handoff_status_label(handoff.status)).bind(to_json(&handoff.payload, "handoff.payload")?).bind(handoff_id.as_str()).execute(self.pool()).await.map_err(|e| ArgoError::Provider(format!("cancel handoff: {e}")))?; self.update_handoff_resolved_at(handoff_id, Utc::now().timestamp_micros()).await?; write_handoff_cancelled_audit(self.pool(), &handoff, note).await?; Ok(handoff)
    }
}
