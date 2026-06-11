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
    ///
    /// Defense-in-depth note: identity files
    /// ([`MemoryFile::is_shared_identity`]: `USER.md` / `SOUL.md` /
    /// `AGENTS.md`) are conceptually **workspace-global** and should be
    /// authored NULL-`vertical_id` via [`Self::append`]. Tagging an
    /// identity row with a non-NULL `vertical_id` here does NOT make it
    /// cross-vertical-readable: the shared-identity read
    /// ([`Self::list_for_vertical_or_shared`]) returns only NULL-vertical
    /// identity rows, so such a row is reachable only by `list_for_vertical`
    /// for that exact vertical — never another vertical.
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

    /// Distinct `vertical_id`s that have at least one non-superseded
    /// entry in this workspace (cu1g · DEC-050). Workspace-scoped, so the
    /// result is inherently tenant-bounded — callers use it to enumerate
    /// only the verticals a tenant actually has data under, instead of
    /// walking every tenant's `.genesis` tree on disk (the O(N·M) DoS the
    /// adversarial review flagged on the approval hot path). One indexed
    /// SELECT; `NULL` (workspace-global) entries are excluded.
    pub async fn distinct_verticals(
        &self,
        workspace_id: WorkspaceId,
        file: MemoryFile,
    ) -> ArgoResult<Vec<String>> {
        self.ensure_layer_schema().await?;
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT DISTINCT m.vertical_id \
             FROM memory_entry m \
             WHERE m.workspace_id = ? AND m.file = ? \
               AND m.vertical_id IS NOT NULL \
               AND m.superseded_by IS NULL",
        )
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(file.filename())
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory distinct_verticals: {e}")))?;
        Ok(rows.into_iter().map(|(v,)| v).collect())
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
    ) -> argo_types::ArgoResult<Vec<(MemoryEntry, crate::MemorySkillCandidate)>> {
        let rows = self
            .list_for_vertical(workspace_id, MemoryFile::Memory, vertical_id)
            .await?;
        Ok(rows
            .into_iter()
            .filter_map(|e| crate::parse_skill_candidate(&e.body).map(|c| (e, c)))
            .collect())
    }

    /// Read active entries for a vertical-bound caller, honouring the
    /// shared-identity exception (adversarial-review follow-up to the
    /// vertical-scoped memory tool, #2787).
    ///
    /// `MEMORY.md` is strictly vertical-isolated: routed through
    /// [`Self::list_for_vertical`], which returns only the bound
    /// vertical's rows and excludes both other verticals' rows AND
    /// legacy NULL-`vertical_id` rows (Rule 7 default-deny). The identity
    /// files (`USER.md` / `SOUL.md` / `AGENTS.md`,
    /// [`MemoryFile::is_shared_identity`]) are workspace-global —
    /// seeded / authored with a NULL `vertical_id` — so a vertical-bound
    /// teammate must still read them; they route through the
    /// NULL-vertical-only [`Self::list_workspace_global`]. "Shared" means
    /// workspace-global ONLY, never "any vertical": a USER.md tagged with a
    /// non-NULL `vertical_id` via [`Self::append_for_vertical`] stays
    /// invisible to other verticals, so this branch does NOT use the
    /// all-vertical [`Self::list_active`]. Tenant scope is preserved in both
    /// branches because a `workspace_id` belongs to exactly one tenant.
    ///
    /// This is the read primitive a vertical-bound agent-runtime memory
    /// tool should call: shared identity stays readable without weakening
    /// `MEMORY.md` isolation.
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # async fn doc() -> argo_types::ArgoResult<()> {
    /// use argo_memory::{MemoryFile, MemoryStore};
    /// use argo_store::SessionStore;
    /// let session = SessionStore::open(":memory:").await?;
    /// session.migrate().await?;
    /// let (_tenant, ws) = session.ensure_workspace("ws", "/tmp/ws").await?;
    /// let store = MemoryStore::new(Arc::new(session));
    /// // Shared identity authored workspace-global (NULL vertical):
    /// store.append(ws, MemoryFile::User, "hello".into(), None).await?;
    /// // A vertical-bound teammate can still read it:
    /// let got = store
    ///     .list_for_vertical_or_shared(ws, MemoryFile::User, "support")
    ///     .await?;
    /// assert_eq!(got.len(), 1);
    /// # Ok(()) }
    /// ```
    pub async fn list_for_vertical_or_shared(
        &self,
        workspace_id: WorkspaceId,
        file: MemoryFile,
        vertical_id: &str,
    ) -> ArgoResult<Vec<MemoryEntry>> {
        if file.is_shared_identity() {
            self.list_workspace_global(workspace_id, file).await
        } else {
            self.list_for_vertical(workspace_id, file, vertical_id)
                .await
        }
    }

    /// List active **workspace-global** entries for `(workspace, file)` —
    /// i.e. `superseded_by IS NULL` AND `vertical_id IS NULL`, in insertion
    /// order.
    ///
    /// This is the read backing the shared-identity branch of
    /// [`Self::list_for_vertical_or_shared`]. It deliberately differs from
    /// the all-vertical [`Self::list_active`]: filtering to
    /// `vertical_id IS NULL` closes a cross-vertical identity-file read. A
    /// USER.md written with a non-NULL `vertical_id` (via
    /// [`Self::append_for_vertical`], reachable from the `--vertical` CLI
    /// flag) is owned by that one vertical and must NOT be returned to a
    /// teammate bound to a different vertical. "Shared" means
    /// workspace-global, never "any vertical". Tenant scope is preserved
    /// because `workspace_id` belongs to exactly one tenant.
    async fn list_workspace_global(
        &self,
        workspace_id: WorkspaceId,
        file: MemoryFile,
    ) -> ArgoResult<Vec<MemoryEntry>> {
        self.ensure_layer_schema().await?;
        let rows = sqlx::query_as::<_, crate::store::RawEntry>(
            "SELECT m.id, m.workspace_id, m.file, m.body, m.learned_in, \
             m.learned_at, m.superseded_by, m.hash, l.lock, m.vertical_id \
             FROM memory_entry m LEFT JOIN memory_lock l ON l.entry_id = m.id \
             WHERE m.workspace_id = ? AND m.file = ? AND m.superseded_by IS NULL \
               AND m.vertical_id IS NULL \
             ORDER BY m.learned_at ASC",
        )
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(file.filename())
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list_workspace_global memory: {e}")))?;
        rows.into_iter().map(MemoryEntry::try_from).collect()
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
