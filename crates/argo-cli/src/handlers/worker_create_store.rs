//! Persistence layer + output types for `worker_create`.
//!
//! Holds the SQL row plumbing, the `WorkerOut` family of serialize
//! structs, and the small JSON helpers used by both insert and load
//! paths. Kept in a sibling module so `worker_create.rs` can stay
//! below the 500-LOC ceiling while the orchestration / CLI-output
//! surface lives there.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub(super) struct PersonaOut {
    pub(super) name: String,
    pub(super) voice_preamble: String,
    pub(super) avatar: Option<String>,
    pub(super) bio: Option<String>,
    pub(super) builder_attribution: String,
    pub(super) provenance_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct DutyOut {
    pub(super) cron: String,
    pub(super) entry_action: String,
    pub(super) description: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct RoleOut {
    pub(super) charter: String,
    pub(super) duties: Vec<DutyOut>,
    pub(super) kpis: Vec<String>,
    pub(super) escalation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct MemoryNsOut {
    pub(super) tenant_id: String,
    pub(super) worker_id: String,
    pub(super) scope: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct WorkerOut {
    pub(super) id: String,
    pub(super) tenant_id: String,
    pub(super) vertical_id: String,
    pub(super) persona: PersonaOut,
    pub(super) role: RoleOut,
    pub(super) memory_namespace: MemoryNsOut,
    pub(super) auto_approval: serde_json::Value,
    pub(super) channels: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) fs_root: Option<String>,
}

pub(super) async fn open_workspace_store() -> ArgoResult<(SessionStore, TenantId, PathBuf)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant_id, _workspace) = store.ensure_workspace(name, &root).await?;
    Ok((store, tenant_id, cwd))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_worker(
    store: &SessionStore,
    tenant_id: TenantId,
    vertical: &str,
    persona: &str,
    charter: &str,
    builder: Option<&str>,
    voice_preamble: Option<&str>,
    duties: &[DutyOut],
    fs_root: Option<&Path>,
) -> ArgoResult<WorkerOut> {
    let worker_id = Uuid::now_v7().to_string();
    let persona_id = Uuid::now_v7().to_string();
    let role_name = format!("worker:{worker_id}");
    let now = Utc::now().timestamp_micros();
    let voice = voice_preamble
        .map(str::to_string)
        .unwrap_or_else(|| format!("You are {persona}, an autonomous SKYCOT worker."));
    let builder = builder.map(str::to_string).unwrap_or_else(default_builder);
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("identity tx: {e}")))?;

    sqlx::query("INSERT INTO persona VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&persona_id)
        .bind(&worker_id)
        .bind(tenant_id.to_string())
        .bind(persona)
        .bind(&voice)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(&builder)
        .bind(json_string(&Vec::<String>::new(), "persona provenance")?)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert persona: {e}")))?;

    sqlx::query(
        "INSERT INTO role (
            tenant_id, name, inherits_from, capabilities, worker_id,
            charter, kpis_json, escalation
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(&role_name)
        .bind(Option::<String>::None)
        .bind("[]")
        .bind(&worker_id)
        .bind(charter)
        .bind(json_string(&Vec::<String>::new(), "role kpis")?)
        .bind(Option::<String>::None)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert role: {e}")))?;

    let fs_root_value = fs_root.map(|p| p.display().to_string());
    sqlx::query(
        "INSERT INTO worker (
            id, tenant_id, vertical_id, persona_id, role_id,
            memory_scope, auto_approval_json, channels_json, created_at, fs_root
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
        .bind(&worker_id)
        .bind(tenant_id.to_string())
        .bind(vertical)
        .bind(&persona_id)
        .bind(&role_name)
        .bind("worker_self")
        .bind(json_string(&serde_json::json!({"enabled": false}), "auto approval")?)
        .bind(json_string(&Vec::<serde_json::Value>::new(), "channels")?)
        .bind(now)
        .bind(fs_root_value.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert worker: {e}")))?;

    for duty in duties {
        sqlx::query("INSERT INTO duty VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(Uuid::now_v7().to_string())
            .bind(&worker_id)
            .bind(&role_name)
            .bind(tenant_id.to_string())
            .bind(&duty.cron)
            .bind(&duty.entry_action)
            .bind(&duty.description)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("insert duty: {e}")))?;
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("identity commit: {e}")))?;
    load_worker(store, tenant_id, &worker_id).await
}

pub(super) async fn load_worker(
    store: &SessionStore,
    tenant_id: TenantId,
    worker_id: &str,
) -> ArgoResult<WorkerOut> {
    let row = sqlx::query(WORKER_SELECT)
        .bind(worker_id)
        .bind(tenant_id.to_string())
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("load worker: {e}")))?
        .ok_or_else(|| ArgoError::Provider(format!("no worker with id '{worker_id}'")))?;
    let duties = load_duties(store, worker_id).await?;
    worker_from_row(row, duties)
}

const WORKER_SELECT: &str = "SELECT
    w.id, w.tenant_id, w.vertical_id, w.memory_scope,
    w.auto_approval_json, w.channels_json, w.fs_root,
    p.name, p.voice_preamble, p.avatar, p.bio, p.builder_attribution,
    p.provenance_chain_json,
    r.charter, r.kpis_json, r.escalation
FROM worker w
JOIN persona p ON p.id = w.persona_id
JOIN role r ON r.worker_id = w.id
WHERE w.id = ? AND w.tenant_id = ?";

async fn load_duties(store: &SessionStore, worker_id: &str) -> ArgoResult<Vec<DutyOut>> {
    let rows = sqlx::query(
        "SELECT cron, entry_action, description
         FROM duty WHERE worker_id = ? ORDER BY created_at ASC, id ASC",
    )
    .bind(worker_id)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("load duties: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| DutyOut {
            cron: row.get("cron"),
            entry_action: row.get("entry_action"),
            description: row.get("description"),
        })
        .collect())
}

fn worker_from_row(row: sqlx::sqlite::SqliteRow, duties: Vec<DutyOut>) -> ArgoResult<WorkerOut> {
    let id: String = row.get("id");
    let tenant_id: String = row.get("tenant_id");
    Ok(WorkerOut {
        id: id.clone(),
        tenant_id: tenant_id.clone(),
        vertical_id: row.get("vertical_id"),
        persona: PersonaOut {
            name: row.get("name"),
            voice_preamble: row.get("voice_preamble"),
            avatar: row.get("avatar"),
            bio: row.get("bio"),
            builder_attribution: row.get("builder_attribution"),
            provenance_chain: json_value(row.get("provenance_chain_json"), "provenance")?,
        },
        role: RoleOut {
            charter: row.get("charter"),
            duties,
            kpis: json_value(row.get("kpis_json"), "kpis")?,
            escalation: row.get("escalation"),
        },
        memory_namespace: MemoryNsOut {
            tenant_id,
            worker_id: id,
            scope: row.get("memory_scope"),
        },
        auto_approval: json_value(row.get("auto_approval_json"), "auto approval")?,
        channels: json_value(row.get("channels_json"), "channels")?,
        fs_root: row.try_get("fs_root").ok().flatten(),
    })
}

fn json_string<T: Serialize>(value: &T, label: &str) -> ArgoResult<String> {
    serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialize {label}: {e}")))
}

fn json_value<T: serde::de::DeserializeOwned>(value: &str, label: &str) -> ArgoResult<T> {
    serde_json::from_str(value).map_err(|e| ArgoError::Provider(format!("parse {label}: {e}")))
}

fn default_builder() -> String {
    std::env::var("ARGO_OPERATOR")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "operator".into())
}
