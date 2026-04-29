//! `MemoryStore` — async wrapper over the `memory_entry` + `memory_lock`
//! tables in argo-store.
use crate::entry::{MemoryEntry, MemoryFile, PrincipalOverride};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock, SessionId, WorkspaceId};
use chrono::{TimeZone, Utc};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Content-addressed memory store with append-only history + locks.
pub struct MemoryStore {
    store: Arc<SessionStore>,
}

impl std::fmt::Debug for MemoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryStore").finish()
    }
}

impl MemoryStore {
    /// Build over an open session store.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Append a brand-new entry for `file`. Use [`Self::supersede`] to
    /// replace an existing entry — that path enforces lock semantics.
    pub async fn append(
        &self,
        workspace_id: WorkspaceId,
        file: MemoryFile,
        body: String,
        learned_in: Option<SessionId>,
    ) -> ArgoResult<MemoryEntry> {
        let id = Uuid::now_v7();
        let learned_at = Utc::now();
        let hash = sha256(&body);
        sqlx::query(
            "INSERT INTO memory_entry (id, workspace_id, file, body, learned_in, \
             learned_at, hash) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(file.filename())
        .bind(&body)
        .bind(learned_in.map(|s| s.0.as_bytes().to_vec()))
        .bind(learned_at.timestamp_micros())
        .bind(hash.as_slice())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory append: {e}")))?;
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
        })
    }

    /// Replace `prev_id` with a new entry carrying `body`. Honours the
    /// previous entry's lock — `UserProtected` requires
    /// [`PrincipalOverride::Force`]; `ContentLocked` requires the new
    /// entry's `superseded_by` link to remain in chain (which we set
    /// here automatically).
    pub async fn supersede(
        &self,
        prev_id: Uuid,
        new_body: String,
        learned_in: Option<SessionId>,
        principal: PrincipalOverride,
    ) -> ArgoResult<MemoryEntry> {
        let prev = self.get(prev_id).await?;
        if matches!(prev.lock, Lock::UserProtected) && principal != PrincipalOverride::Force {
            return Err(ArgoError::UserProtected {
                entity: format!("memory_entry:{prev_id}"),
            });
        }
        let new_entry = self
            .append(prev.workspace_id, prev.file, new_body, learned_in)
            .await?;
        sqlx::query("UPDATE memory_entry SET superseded_by = ? WHERE id = ?")
            .bind(new_entry.id.as_bytes().as_slice())
            .bind(prev_id.as_bytes().as_slice())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("memory supersede: {e}")))?;
        Ok(new_entry)
    }

    /// Revert `entry_id` by writing a new entry whose body is the prior
    /// version's body, then linking the current head's `superseded_by`
    /// to the new entry. Lock semantics still apply.
    pub async fn revert(
        &self,
        entry_id: Uuid,
        principal: PrincipalOverride,
    ) -> ArgoResult<MemoryEntry> {
        let target = self.get(entry_id).await?;
        // Find the current head (active entry) for this file.
        let active = self
            .list_active(target.workspace_id, target.file)
            .await?
            .into_iter()
            .find(|e| e.id != target.id);
        let body = target.body.clone();
        match active {
            Some(head) => self.supersede(head.id, body, None, principal).await,
            None => {
                self.append(target.workspace_id, target.file, body, None)
                    .await
            }
        }
    }

    /// Set or clear the lock on an entry. Only [`PrincipalOverride::Force`]
    /// (a user-driven principal) may set a lock — agents cannot lock
    /// themselves out or escalate by removing one.
    pub async fn set_lock(
        &self,
        entry_id: Uuid,
        lock: Lock,
        principal: PrincipalOverride,
    ) -> ArgoResult<()> {
        if principal != PrincipalOverride::Force {
            return Err(ArgoError::UserProtected {
                entity: format!("memory_lock:{entry_id} (only user can set/clear)"),
            });
        }
        match lock {
            Lock::None => {
                sqlx::query("DELETE FROM memory_lock WHERE entry_id = ?")
                    .bind(entry_id.as_bytes().as_slice())
                    .execute(self.store.pool())
                    .await
                    .map_err(|e| ArgoError::Provider(format!("memory unlock: {e}")))?;
            }
            other => {
                let label = match other {
                    Lock::UserProtected => "user-protected",
                    Lock::ContentLocked => "content-locked",
                    Lock::None => unreachable!(),
                };
                sqlx::query(
                    "INSERT INTO memory_lock (entry_id, lock, set_at, set_by) \
                     VALUES (?, ?, ?, 'user') \
                     ON CONFLICT(entry_id) DO UPDATE SET lock = excluded.lock, \
                     set_at = excluded.set_at, set_by = 'user'",
                )
                .bind(entry_id.as_bytes().as_slice())
                .bind(label)
                .bind(Utc::now().timestamp_micros())
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("memory lock: {e}")))?;
            }
        }
        Ok(())
    }

    /// Active entries for `(workspace, file)` (i.e. `superseded_by IS NULL`),
    /// in insertion order.
    pub async fn list_active(
        &self,
        workspace_id: WorkspaceId,
        file: MemoryFile,
    ) -> ArgoResult<Vec<MemoryEntry>> {
        let rows = sqlx::query_as::<_, RawEntry>(
            "SELECT m.id, m.workspace_id, m.file, m.body, m.learned_in, \
             m.learned_at, m.superseded_by, m.hash, l.lock \
             FROM memory_entry m LEFT JOIN memory_lock l ON l.entry_id = m.id \
             WHERE m.workspace_id = ? AND m.file = ? AND m.superseded_by IS NULL \
             ORDER BY m.learned_at ASC",
        )
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(file.filename())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list_active memory: {e}")))?;
        rows.into_iter().map(MemoryEntry::try_from).collect()
    }

    /// Fetch a single entry by id (active or superseded).
    pub async fn get(&self, entry_id: Uuid) -> ArgoResult<MemoryEntry> {
        let row = sqlx::query_as::<_, RawEntry>(
            "SELECT m.id, m.workspace_id, m.file, m.body, m.learned_in, \
             m.learned_at, m.superseded_by, m.hash, l.lock \
             FROM memory_entry m LEFT JOIN memory_lock l ON l.entry_id = m.id \
             WHERE m.id = ?",
        )
        .bind(entry_id.as_bytes().as_slice())
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory get: {e}")))?
        .ok_or_else(|| ArgoError::Schema(format!("memory entry {entry_id} not found")))?;
        MemoryEntry::try_from(row)
    }
}

fn sha256(body: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

#[derive(sqlx::FromRow)]
struct RawEntry {
    id: Vec<u8>,
    workspace_id: Vec<u8>,
    file: String,
    body: String,
    learned_in: Option<Vec<u8>>,
    learned_at: i64,
    superseded_by: Option<Vec<u8>>,
    hash: Vec<u8>,
    lock: Option<String>,
}

impl TryFrom<RawEntry> for MemoryEntry {
    type Error = ArgoError;
    fn try_from(r: RawEntry) -> ArgoResult<Self> {
        let id = uuid_from_blob(&r.id, "memory_entry.id")?;
        let workspace_id = WorkspaceId(uuid_from_blob(
            &r.workspace_id,
            "memory_entry.workspace_id",
        )?);
        let file = MemoryFile::parse(&r.file).ok_or_else(|| {
            ArgoError::Schema(format!("memory_entry.file: unknown filename {}", r.file))
        })?;
        let learned_in = r
            .learned_in
            .map(|b| uuid_from_blob(&b, "memory_entry.learned_in"))
            .transpose()?
            .map(SessionId);
        let learned_at = Utc
            .timestamp_micros(r.learned_at)
            .single()
            .ok_or_else(|| ArgoError::Schema("memory_entry.learned_at out of range".into()))?;
        let superseded_by = r
            .superseded_by
            .map(|b| uuid_from_blob(&b, "memory_entry.superseded_by"))
            .transpose()?;
        let mut hash = [0u8; 32];
        if r.hash.len() != 32 {
            return Err(ArgoError::Schema(
                "memory_entry.hash must be 32 bytes".into(),
            ));
        }
        hash.copy_from_slice(&r.hash);
        let lock = match r.lock.as_deref() {
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
            body: r.body,
            learned_in,
            learned_at,
            superseded_by,
            hash,
            lock,
        })
    }
}

fn uuid_from_blob(b: &[u8], field: &'static str) -> ArgoResult<Uuid> {
    Uuid::from_slice(b).map_err(|e| ArgoError::Schema(format!("{field}: {e}")))
}
