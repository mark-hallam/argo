use crate::{circuit, CircuitState, Duty, MemoryNs, MemoryScope, Persona, Role, VoiceProfile, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

const IDENTITY_SCHEMA: &str = include_str!("../../argo-store/migrations/12__worker_identity.sql");

/// SQLite-backed persistence for worker identities.
///
/// ```
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// use argo_identity::{IdentityStore, Persona, Role, Worker};
/// use argo_store::SessionStore;
/// use argo_types::TenantId;
///
/// let store = SessionStore::open(":memory:").await?;
/// store.migrate().await?;
/// let identities = IdentityStore::new(&store);
/// identities.ensure_schema().await?;
/// let worker = Worker::draft(TenantId::new(), "gtm", Persona::named("Maya", "op"), Role::new("Sell."));
/// identities.create_worker(&worker).await?;
/// # Ok(()) }
/// ```
pub struct IdentityStore<'a> {
    store: &'a SessionStore,
}

impl<'a> IdentityStore<'a> {
    /// Wrap an existing session store.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::IdentityStore;
    /// use argo_store::SessionStore;
    /// let store = SessionStore::open(":memory:").await?;
    /// let _identities = IdentityStore::new(&store);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn new(store: &'a SessionStore) -> Self {
        Self { store }
    }

    /// Create identity tables if they are missing.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::IdentityStore;
    /// use argo_store::SessionStore;
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// IdentityStore::new(&store).ensure_schema().await?;
    /// # Ok(()) }
    /// ```
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in IDENTITY_SCHEMA.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("identity schema: {e}")))?;
        }
        ensure_role_columns(self.store.pool()).await?;
        Ok(())
    }

    /// Persist a worker, its persona, role, and duties.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{IdentityStore, Persona, Role, Worker};
    /// use argo_store::SessionStore;
    /// use argo_types::TenantId;
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let identities = IdentityStore::new(&store);
    /// let worker = Worker::draft(TenantId::new(), "gtm", Persona::named("Maya", "op"), Role::new("Sell."));
    /// identities.create_worker(&worker).await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_worker(&self, worker: &Worker) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let persona_id = Uuid::now_v7().to_string();
        let role_name = format!("worker:{}", worker.id);
        let now = Utc::now().timestamp_micros();
        let mut tx = self
            .store
            .pool()
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("identity tx: {e}")))?;

        sqlx::query(
            "INSERT INTO persona (
                id, worker_id, tenant_id, name, voice_preamble, avatar, bio,
                builder_attribution, provenance_chain_json
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&persona_id)
        .bind(worker.id.to_string())
        .bind(worker.tenant_id.to_string())
        .bind(&worker.persona.name)
        .bind(&worker.persona.voice_profile.preamble)
        .bind(&worker.persona.avatar)
        .bind(&worker.persona.bio)
        .bind(&worker.persona.builder_attribution)
        .bind(to_json(&worker.persona.provenance_chain, "persona provenance")?)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert persona: {e}")))?;

        sqlx::query(
            "INSERT INTO role (
                tenant_id, name, inherits_from, capabilities, worker_id,
                charter, kpis_json, escalation
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(worker.tenant_id.0.as_bytes().as_slice())
        .bind(&role_name)
        .bind(Option::<String>::None)
        .bind("[]")
        .bind(worker.id.to_string())
        .bind(&worker.role.charter)
        .bind(to_json(&worker.role.kpis, "role kpis")?)
        .bind(&worker.role.escalation)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert role: {e}")))?;

        sqlx::query(
            "INSERT INTO worker (
                id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
                auto_approval_json, channels_json, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(worker.id.to_string())
        .bind(worker.tenant_id.to_string())
        .bind(&worker.vertical_id)
        .bind(&persona_id)
        .bind(&role_name)
        .bind(worker.memory_namespace.scope.as_str())
        .bind(to_json(&worker.auto_approval, "auto approval")?)
        .bind(to_json(&worker.channels, "channels")?)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert worker: {e}")))?;

        for duty in &worker.role.duties {
            insert_duty(&mut tx, worker, &role_name, duty, now).await?;
        }
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("identity commit: {e}")))?;
        Ok(())
    }

    /// Load one worker for a tenant.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{IdentityStore, Persona, Role, Worker};
    /// use argo_store::SessionStore;
    /// use argo_types::TenantId;
    /// let tenant = TenantId::new();
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let identities = IdentityStore::new(&store);
    /// let worker = Worker::draft(tenant, "gtm", Persona::named("Maya", "op"), Role::new("Sell."));
    /// identities.create_worker(&worker).await?;
    /// assert!(identities.get_worker(tenant, worker.id).await?.is_some());
    /// # Ok(()) }
    /// ```
    pub async fn get_worker(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<Option<Worker>> {
        self.ensure_schema().await?;
        let row = sqlx::query(WORKER_SELECT)
            .bind(worker_id.to_string())
            .bind(tenant_id.to_string())
            .fetch_optional(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("get worker: {e}")))?;
        match row {
            Some(row) => {
                let duties = self.load_duties(worker_id).await?;
                let mut worker = worker_from_row(row, duties)?;
                worker.circuit_state = circuit::state_for(
                    self.store.pool(),
                    &worker.id.to_string(),
                )
                .await?;
                Ok(Some(worker))
            }
            None => Ok(None),
        }
    }

    /// List workers for one tenant, newest first.
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{IdentityStore, Persona, Role, Worker};
    /// use argo_store::SessionStore;
    /// use argo_types::TenantId;
    /// let tenant = TenantId::new();
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let identities = IdentityStore::new(&store);
    /// identities.create_worker(&Worker::draft(tenant, "gtm", Persona::named("Maya", "op"), Role::new("Sell."))).await?;
    /// assert_eq!(identities.list_workers(tenant).await?.len(), 1);
    /// # Ok(()) }
    /// ```
    pub async fn list_workers(&self, tenant_id: TenantId) -> ArgoResult<Vec<Worker>> {
        self.ensure_schema().await?;
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM worker WHERE tenant_id = ? ORDER BY created_at DESC, id ASC",
        )
        .bind(tenant_id.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list workers: {e}")))?;

        let mut workers = Vec::with_capacity(ids.len());
        for id in ids {
            let worker_id = parse_worker_id(&id)?;
            if let Some(worker) = self.get_worker(tenant_id, worker_id).await? {
                workers.push(worker);
            }
        }
        Ok(workers)
    }

    async fn load_duties(&self, worker_id: WorkerId) -> ArgoResult<Vec<Duty>> {
        let rows = sqlx::query(
            "SELECT cron, entry_action, description
             FROM duty WHERE worker_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(worker_id.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("load duties: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|row| Duty {
                cron: row.get("cron"),
                entry_action: row.get("entry_action"),
                description: row.get("description"),
            })
            .collect())
    }
}

const WORKER_SELECT: &str = "SELECT
    w.id, w.tenant_id, w.vertical_id, w.memory_scope,
    w.auto_approval_json, w.channels_json,
    p.name, p.voice_preamble, p.avatar, p.bio, p.builder_attribution,
    p.provenance_chain_json,
    r.charter, r.kpis_json, r.escalation
FROM worker w
JOIN persona p ON p.id = w.persona_id
JOIN role r ON r.worker_id = w.id
WHERE w.id = ? AND w.tenant_id = ?";

async fn ensure_role_columns(pool: &sqlx::Pool<sqlx::Sqlite>) -> ArgoResult<()> {
    let rows = sqlx::query("PRAGMA table_info(role)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("role table info: {e}")))?;
    let columns: Vec<String> = rows.into_iter().map(|row| row.get("name")).collect();
    for (name, ty) in [
        ("worker_id", "TEXT"),
        ("charter", "TEXT"),
        ("kpis_json", "TEXT"),
        ("escalation", "TEXT"),
    ] {
        if !columns.iter().any(|existing| existing == name) {
            let sql = format!("ALTER TABLE role ADD COLUMN {name} {ty}");
            sqlx::query(&sql)
                .execute(pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("add role.{name}: {e}")))?;
        }
    }
    Ok(())
}

async fn insert_duty(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    worker: &Worker,
    role_id: &str,
    duty: &Duty,
    now: i64,
) -> ArgoResult<()> {
    sqlx::query(
        "INSERT INTO duty (
            id, worker_id, role_id, tenant_id, cron, entry_action, description, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(worker.id.to_string())
    .bind(role_id)
    .bind(worker.tenant_id.to_string())
    .bind(&duty.cron)
    .bind(&duty.entry_action)
    .bind(&duty.description)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("insert duty: {e}")))?;
    Ok(())
}

fn worker_from_row(row: sqlx::sqlite::SqliteRow, duties: Vec<Duty>) -> ArgoResult<Worker> {
    let id = parse_worker_id(row.get("id"))?;
    let tenant_id = parse_tenant_id(row.get("tenant_id"))?;
    let scope = MemoryScope::parse(row.get("memory_scope"))
        .ok_or_else(|| ArgoError::Provider("unknown memory scope".into()))?;
    let persona = Persona {
        name: row.get("name"),
        voice_profile: VoiceProfile {
            preamble: row.get("voice_preamble"),
        },
        avatar: row.get("avatar"),
        bio: row.get("bio"),
        builder_attribution: row.get("builder_attribution"),
        provenance_chain: from_json(row.get("provenance_chain_json"), "persona provenance")?,
    };
    let role = Role {
        charter: row.get("charter"),
        duties,
        kpis: from_json(row.get("kpis_json"), "role kpis")?,
        escalation: row.get("escalation"),
    };
    Ok(Worker {
        id,
        tenant_id,
        vertical_id: row.get("vertical_id"),
        memory_namespace: MemoryNs::new(tenant_id, id, scope),
        persona,
        role,
        auto_approval: from_json(row.get("auto_approval_json"), "auto approval")?,
        circuit_state: CircuitState::Closed,
        channels: from_json(row.get("channels_json"), "channels")?,
    })
}

fn to_json<T: Serialize>(value: &T, label: &str) -> ArgoResult<String> {
    serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialize {label}: {e}")))
}

fn from_json<T: serde::de::DeserializeOwned>(value: &str, label: &str) -> ArgoResult<T> {
    serde_json::from_str(value).map_err(|e| ArgoError::Provider(format!("parse {label}: {e}")))
}

fn parse_worker_id(value: &str) -> ArgoResult<WorkerId> {
    value
        .parse()
        .map_err(|e| ArgoError::Provider(format!("worker id '{value}': {e}")))
}

fn parse_tenant_id(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_get_list_roundtrip() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let tenant = TenantId::new();
        let mut role = Role::new("Own outbound follow-up.");
        role.duties
            .push(Duty::new("0 9 * * 1-5", "follow_up", "Send approved follow-up."));
        let worker = Worker::draft(tenant, "gtm-engineer", Persona::named("Maya", "op"), role);
        let identities = IdentityStore::new(&store);

        identities.create_worker(&worker).await.unwrap();
        let loaded = identities
            .get_worker(tenant, worker.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.persona.name, "Maya");
        assert_eq!(loaded.role.duties[0].entry_action, "follow_up");
        assert_eq!(identities.list_workers(tenant).await.unwrap().len(), 1);
    }
}
