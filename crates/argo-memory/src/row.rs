//! Row conversion helpers for layered memory queries.
use crate::{MemoryEntry, MemoryFile};
use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, TenantId, WorkspaceId};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub(crate) struct LayerRow {
    pub(crate) id: Vec<u8>,
    pub(crate) workspace_id: Vec<u8>,
    pub(crate) file: String,
    pub(crate) body: String,
    pub(crate) learned_in: Option<Vec<u8>>,
    pub(crate) learned_at: i64,
    pub(crate) superseded_by: Option<Vec<u8>>,
    pub(crate) hash: Vec<u8>,
    pub(crate) lock: Option<String>,
    pub(crate) vertical_id: Option<String>,
    pub(crate) layer: String,
    pub(crate) vector_json: Option<String>,
}

impl LayerRow {
    pub(crate) fn into_memory_entry(self) -> ArgoResult<MemoryEntry> {
        let id = uuid_from_blob(&self.id, "memory_entry.id")?;
        let workspace_id = WorkspaceId(uuid_from_blob(
            &self.workspace_id,
            "memory_entry.workspace_id",
        )?);
        let file = MemoryFile::parse(&self.file).ok_or_else(|| {
            ArgoError::Schema(format!("memory_entry.file: unknown filename {}", self.file))
        })?;
        let learned_in = self
            .learned_in
            .map(|b| uuid_from_blob(&b, "memory_entry.learned_in"))
            .transpose()?
            .map(SessionId);
        let learned_at = Utc
            .timestamp_micros(self.learned_at)
            .single()
            .ok_or_else(|| ArgoError::Schema("memory_entry.learned_at out of range".into()))?;
        let superseded_by = self
            .superseded_by
            .map(|b| uuid_from_blob(&b, "memory_entry.superseded_by"))
            .transpose()?;
        let mut hash = [0_u8; 32];
        if self.hash.len() != 32 {
            return Err(ArgoError::Schema(
                "memory_entry.hash must be 32 bytes".into(),
            ));
        }
        hash.copy_from_slice(&self.hash);
        let lock = match self.lock.as_deref() {
            None => Lock::None,
            Some("user-protected") => Lock::UserProtected,
            Some("content-locked") => Lock::ContentLocked,
            Some(other) => {
                return Err(ArgoError::Schema(format!(
                    "memory_lock.lock: unknown variant {other}"
                )))
            }
        };
        Ok(MemoryEntry {
            id,
            workspace_id,
            file,
            body: self.body,
            learned_in,
            learned_at,
            superseded_by,
            hash,
            lock,
            vertical_id: self.vertical_id,
        })
    }
}

pub(crate) fn scope_as_str(scope: MemoryScope) -> &'static str {
    match scope {
        MemoryScope::WorkerSelf => "worker_self",
        MemoryScope::Role => "role",
        MemoryScope::SharedTeam => "shared_team",
    }
}

pub(crate) fn scope_from_str(scope: &str) -> ArgoResult<MemoryScope> {
    match scope {
        "worker_self" => Ok(MemoryScope::WorkerSelf),
        "role" => Ok(MemoryScope::Role),
        "shared_team" => Ok(MemoryScope::SharedTeam),
        other => Err(ArgoError::Schema(format!(
            "memory_entry.memory_scope: unknown scope {other}"
        ))),
    }
}

pub(crate) fn ns_from_parts(
    tenant_id: &[u8],
    worker_id: &[u8],
    scope: &str,
) -> ArgoResult<MemoryNs> {
    Ok(MemoryNs::new(
        TenantId(uuid_from_blob(tenant_id, "memory_entry.tenant_id")?),
        WorkerId(uuid_from_blob(worker_id, "memory_entry.worker_id")?),
        scope_from_str(scope)?,
    ))
}

pub(crate) fn uuid_from_blob(b: &[u8], field: &'static str) -> ArgoResult<Uuid> {
    Uuid::from_slice(b).map_err(|e| ArgoError::Schema(format!("{field}: {e}")))
}
