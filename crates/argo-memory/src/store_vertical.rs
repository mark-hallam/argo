//! Vertical-scoped memory operations extracted from `store.rs` to keep
//! that file under the 500-LOC ceiling (HARD RULE 6).
//!
//! These methods enforce strict per-vertical isolation (#680, Rule 7
//! default-deny): writes are tagged with the bound vertical, reads
//! return only entries whose `vertical_id` matches. Cross-vertical
//! sharing requires the shared-asset surface (Rule 11, future PR).
//!
//! No public API changes — methods stay on `MemoryStore` via a
//! sibling `impl` block.
use crate::{MemoryEntry, MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, WorkspaceId};
use chrono::{TimeZone, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

impl MemoryStore {
    /// Append a memory entry tagged with the bound vertical
    /// (#680, Rule 7). Reads via `list_for_vertical(_, _, vid)`
    /// will return only entries where `vertical_id = vid`. The
    /// bare `list_active` keeps the existing workspace-scoped
    /// behaviour for backward compat — gateway / CLI integration
    /// into the vertical-aware path is a follow-up.
    pub async fn append_for_vertical(
        &self,
        workspace_id: WorkspaceId,
        file: MemoryFile,
        body: String,
        learned_in: Option<SessionId>,
        vertical_id: &str,
    ) -> ArgoResult<MemoryEntry> {
        self.ensure_layer_schema().await?;
        let id = Uuid::now_v7();
        let learned_at = Utc::now();
        let hash = sha256_body(&body);
        sqlx::query(
            "INSERT INTO memory_entry (id, workspace_id, file, body, learned_in, \
             learned_at, hash, vertical_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(file.filename())
        .bind(&body)
        .bind(learned_in.map(|s| s.0.as_bytes().to_vec()))
        .bind(learned_at.timestamp_micros())
        .bind(hash.as_slice())
        .bind(vertical_id)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory append_for_vertical: {e}")))?;
        Ok(MemoryEntry {
            id,
            workspace_id,
            file,
            body,
            learned_in,
            learned_at,
            superseded_by: None,
            hash,
            lock: Lock::None,
            vertical_id: Some(vertical_id.to_string()),
        })
    }

    /// List active entries scoped to a vertical (#680).
    /// Returns ONLY entries where `vertical_id = ?` AND not
    /// superseded. Excludes legacy NULL entries (default-deny
    /// per Rule 7). Cross-vertical sharing requires the
    /// shared-asset surface (Rule 11, future PR).
    pub async fn list_for_vertical(
        &self,
        workspace_id: WorkspaceId,
        file: MemoryFile,
        vertical_id: &str,
    ) -> ArgoResult<Vec<MemoryEntry>> {
        self.ensure_layer_schema().await?;
        let rows = sqlx::query_as::<
            _,
            (
                Vec<u8>,
                Vec<u8>,
                String,
                String,
                Option<Vec<u8>>,
                i64,
                Option<Vec<u8>>,
                Vec<u8>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT m.id, m.workspace_id, m.file, m.body, m.learned_in, \
                    m.learned_at, m.superseded_by, m.hash, l.lock, m.vertical_id \
             FROM memory_entry m \
             LEFT JOIN memory_lock l ON l.entry_id = m.id \
             WHERE m.workspace_id = ? AND m.file = ? \
               AND m.superseded_by IS NULL \
               AND m.vertical_id = ? \
             ORDER BY m.learned_at ASC",
        )
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(file.filename())
        .bind(vertical_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory list_for_vertical: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for (
            id_b,
            ws_b,
            _file_str,
            body,
            learned_in_b,
            learned_at,
            superseded_b,
            hash_b,
            lock_str,
            vert,
        ) in rows
        {
            let id = Uuid::from_slice(&id_b)
                .map_err(|e| ArgoError::Provider(format!("parse memory.id: {e}")))?;
            let workspace_id = WorkspaceId(
                Uuid::from_slice(&ws_b)
                    .map_err(|e| ArgoError::Provider(format!("parse workspace_id: {e}")))?,
            );
            let learned_in = learned_in_b
                .map(|b| {
                    Uuid::from_slice(&b)
                        .map_err(|e| ArgoError::Provider(format!("parse learned_in: {e}")))
                })
                .transpose()?
                .map(SessionId);
            let superseded_by = superseded_b
                .map(|b| {
                    Uuid::from_slice(&b)
                        .map_err(|e| ArgoError::Provider(format!("parse superseded_by: {e}")))
                })
                .transpose()?;
            let learned_at = Utc
                .timestamp_micros(learned_at)
                .single()
                .ok_or_else(|| ArgoError::Schema("learned_at out of range".into()))?;
            let mut hash = [0u8; 32];
            if hash_b.len() != 32 {
                return Err(ArgoError::Schema("hash must be 32 bytes".into()));
            }
            hash.copy_from_slice(&hash_b);
            let lock = match lock_str.as_deref() {
                None => Lock::None,
                Some("user-protected") => Lock::UserProtected,
                Some("content-locked") => Lock::ContentLocked,
                Some(other) => {
                    return Err(ArgoError::Schema(format!(
                        "memory_lock.lock unknown variant: {other}"
                    )));
                }
            };
            out.push(MemoryEntry {
                id,
                workspace_id,
                file,
                body,
                learned_in,
                learned_at,
                superseded_by,
                hash,
                lock,
                vertical_id: vert,
            });
        }
        Ok(out)
    }

    /// List vertical-scoped MEMORY.md entries whose body parses
    /// as a learned skill candidate (#697). Pure read — joins
    /// `list_for_vertical(MemoryFile::Memory)` with the
    /// JSON-in-body convention parser. Returns each matching
    /// entry paired with its parsed candidate so callers don't
    /// re-parse downstream.
    ///
    /// Strict isolation: only entries with `vertical_id = ?`
    /// AND not superseded are returned (Rule 7 default-deny).
    /// Verticals never see candidates from other verticals.
    pub async fn list_skill_candidates_for_vertical(
        &self,
        workspace_id: argo_types::WorkspaceId,
        vertical_id: &str,
    ) -> argo_types::ArgoResult<Vec<(MemoryEntry, crate::SkillCandidate)>> {
        let rows = self
            .list_for_vertical(workspace_id, MemoryFile::Memory, vertical_id)
            .await?;
        Ok(rows
            .into_iter()
            .filter_map(|e| crate::parse_skill_candidate(&e.body).map(|c| (e, c)))
            .collect())
    }
}

fn sha256_body(body: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}
