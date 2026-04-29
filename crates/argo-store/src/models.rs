//! Read-side row models and conversions.
use argo_types::{SessionId, TenantId, WorkspaceId};
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
