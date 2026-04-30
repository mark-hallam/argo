//! Read-side row models and conversions.
use argo_types::{
    ProviderModel, RoutingDecision, RoutingDecisionId, SessionId, TenantId, WorkspaceId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A session record returned from `SessionStore::get_session` etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    /// Session id.
    pub id: SessionId,
    /// Tenant id.
    pub tenant_id: TenantId,
    /// Workspace id.
    pub workspace_id: WorkspaceId,
    /// Model used.
    pub model: String,
    /// Provider used.
    pub provider: String,
    /// Optional title.
    pub title: Option<String>,
    /// Start time.
    pub started_at: DateTime<Utc>,
    /// Optional end time.
    pub ended_at: Option<DateTime<Utc>>,
}

/// Raw row shape used internally by sqlx.
#[derive(sqlx::FromRow)]
pub(crate) struct SessionHeaderRow {
    pub id: Vec<u8>,
    pub tenant_id: Vec<u8>,
    pub workspace_id: Vec<u8>,
    pub model: String,
    pub provider: String,
    pub title: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

pub(crate) fn uuid_from_blob(b: &[u8]) -> Uuid {
    let mut a = [0u8; 16];
    let n = b.len().min(16);
    a[..n].copy_from_slice(&b[..n]);
    Uuid::from_bytes(a)
}

/// One `audit_record` row exposed for read-side rendering.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct AuditRow {
    /// Append-only sequence number (monotonic across the DB).
    pub seq: i64,
    /// Event kind label.
    pub kind: String,
    /// Canonical payload JSON.
    pub payload_json: String,
    /// `created_at` (UTC, epoch micros).
    pub created_at: i64,
}

/// One FTS5 hit on the message corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSearchHit {
    /// Session the hit belongs to.
    pub session_id: SessionId,
    /// Per-session sequence number.
    pub seq: i64,
    /// Role of the message (`user`/`assistant`/`tool`).
    pub role: String,
    /// FTS5-rendered snippet with `[match]` highlights.
    pub snippet: String,
}

/// Raw FTS5 row.
#[derive(sqlx::FromRow)]
pub(crate) struct MessageSearchHitRow {
    pub session_id: Vec<u8>,
    pub seq: i64,
    pub role: String,
    pub snippet: String,
}

impl From<MessageSearchHitRow> for MessageSearchHit {
    fn from(r: MessageSearchHitRow) -> Self {
        Self {
            session_id: SessionId(uuid_from_blob(&r.session_id)),
            seq: r.seq,
            role: r.role,
            snippet: r.snippet,
        }
    }
}

/// One `message` row. Surfaced as-is for `argo session export`.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct MessageRow {
    /// Message id (uuid blob).
    pub id: Vec<u8>,
    /// Session id (uuid blob).
    pub session_id: Vec<u8>,
    /// Role (`system`/`user`/`assistant`/`tool`).
    pub role: String,
    /// JSON-encoded content blocks.
    pub content_json: String,
    /// JSON-encoded tool calls.
    pub tool_calls_json: String,
    /// Tool-call id (uuid blob) when role is `tool`.
    pub tool_call_id: Option<Vec<u8>>,
    /// Reasoning content (for thinking-token providers).
    pub reasoning: Option<String>,
    /// Provider-specific metadata (JSON).
    pub provider_meta: String,
    /// Created-at timestamp (UTC, micros).
    pub created_at: i64,
    /// Per-session sequence number.
    pub seq: i64,
}

/// One Forge run row. Surfaced as-is for `argo forge runs list`.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ForgeRunRow {
    /// Skill name being optimized.
    pub skill_name: String,
    /// Start timestamp (UTC, micros).
    pub started_at: i64,
    /// End timestamp; `None` while running.
    pub ended_at: Option<i64>,
    /// Base skill version.
    pub base_version: String,
    /// Candidate version (set on success/partial).
    pub candidate_version: Option<String>,
    /// Iteration count actually run.
    pub iterations: i64,
    /// Spend in USD-micros.
    pub spend_usd_micros: i64,
    /// Final score in `[0.0, 1.0]` if known.
    pub final_score: Option<f64>,
    /// Status string (`pending` / `running` / `succeeded` /
    /// `partial` / `failed` / `aborted` per §26.4).
    pub status: String,
}

/// Raw `routing_decision` row.
#[derive(sqlx::FromRow)]
pub(crate) struct RoutingDecisionRow {
    pub id: Vec<u8>,
    pub session_id: Vec<u8>,
    pub turn: i64,
    pub policy: String,
    pub tier: String,
    pub classifier_latency_ms: i64,
    pub chosen_provider: String,
    pub chosen_model: String,
    pub reason: Option<String>,
    pub decided_at: i64,
}

impl From<RoutingDecisionRow> for RoutingDecision {
    fn from(r: RoutingDecisionRow) -> Self {
        Self {
            id: RoutingDecisionId(uuid_from_blob(&r.id)),
            session_id: SessionId(uuid_from_blob(&r.session_id)),
            turn: u32::try_from(r.turn).unwrap_or(0),
            policy: r.policy,
            tier: r.tier,
            classifier_latency_ms: u64::try_from(r.classifier_latency_ms).unwrap_or(0),
            chosen_provider_model: ProviderModel {
                provider: r.chosen_provider,
                model: r.chosen_model,
            },
            reason: r.reason.unwrap_or_default(),
            classified_at: DateTime::<Utc>::from_timestamp_micros(r.decided_at)
                .unwrap_or_else(Utc::now),
        }
    }
}

impl From<SessionHeaderRow> for SessionHeader {
    fn from(r: SessionHeaderRow) -> Self {
        Self {
            id: SessionId(uuid_from_blob(&r.id)),
            tenant_id: TenantId(uuid_from_blob(&r.tenant_id)),
            workspace_id: WorkspaceId(uuid_from_blob(&r.workspace_id)),
            model: r.model,
            provider: r.provider,
            title: r.title,
            started_at: DateTime::<Utc>::from_timestamp_micros(r.started_at)
                .unwrap_or_else(Utc::now),
            ended_at: r.ended_at.and_then(DateTime::<Utc>::from_timestamp_micros),
        }
    }
}
