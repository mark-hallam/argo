//! Worker identity create/show/list CLI.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_cron::{CronExpr, CronJob, CronTarget};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const IDENTITY_SCHEMA: &str = include_str!("../../../argo-store/migrations/12__worker_identity.sql");
const FS_ROOT_SCHEMA: &str = include_str!("../../../argo-store/migrations/19__worker_fs_root.sql");

#[derive(Debug, Clone, Serialize)]
struct PersonaOut {
    name: String,
    voice_preamble: String,
    avatar: Option<String>,
    bio: Option<String>,
    builder_attribution: String,
    provenance_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DutyOut {
    cron: String,
    entry_action: String,
    description: String,
}

#[derive(Debug, Clone, Serialize)]
struct RoleOut {
    charter: String,
    duties: Vec<DutyOut>,
    kpis: Vec<String>,
    escalation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MemoryNsOut {
    tenant_id: String,
    worker_id: String,
    scope: String,
}

#[derive(Debug, Clone, Serialize)]
struct WorkerOut {
    id: String,
    tenant_id: String,
    vertical_id: String,
    persona: PersonaOut,
    role: RoleOut,
    memory_namespace: MemoryNsOut,
    auto_approval: serde_json::Value,
    channels: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    fs_root: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    vertical: &str,
    persona: &str,
    charter: &str,
    builder: Option<&str>,
    voice_preamble: Option<&str>,
    duty_specs: &[String],
    fs_root: Option<&Path>,
    json: bool,
) -> ArgoResult<()> {
    let (store, tenant_id, cwd) = open_workspace_store().await?;
    ensure_identity_schema(&store).await?;
    let duties = parse_duties(duty_specs)?;
    let worker = insert_worker(
        &store,
        tenant_id,
        vertical,
        persona,
        charter,
        builder,
        voice_preamble,
        &duties,
        fs_root,
    )
    .await?;
    register_cron_duties(&cwd, tenant_id, &worker.id, &duties)?;
    print_worker(&worker, json)?;
    Ok(())
}

pub async fn show(worker_id: &str, json: bool) -> ArgoResult<()> {
    let (store, tenant_id, _cwd) = open_workspace_store().await?;
    ensure_identity_schema(&store).await?;
    let worker = load_worker(&store, tenant_id, worker_id).await?;
    print_worker(&worker, json)
}

pub async fn list(json: bool) -> ArgoResult<()> {
    let (store, tenant_id, _cwd) = open_workspace_store().await?;
    ensure_identity_schema(&store).await?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM worker WHERE tenant_id = ? ORDER BY created_at DESC, id ASC",
    )
    .bind(tenant_id.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list workers: {e}")))?;
    let mut workers = Vec::with_capacity(ids.len());
    for id in ids {
        workers.push(load_worker(&store, tenant_id, &id).await?);
    }
    if json {
        print_json(&workers)
    } else {
        if workers.is_empty() {
            println!("(no workers)");
            return Ok(());
        }
        println!("worker_id                  vertical        persona");
        for worker in workers {
            println!(
                "{:<26} {:<15} {}",
                short(&worker.id),
                worker.vertical_id,
                worker.persona.name
            );
        }
        Ok(())
    }
}

async fn open_workspace_store() -> ArgoResult<(SessionStore, TenantId, PathBuf)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
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

async fn ensure_identity_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in IDENTITY_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("identity schema: {e}")))?;
    }
    ensure_role_columns(store).await?;
    ensure_worker_columns(store).await?;
    Ok(())
}

async fn ensure_worker_columns(store: &SessionStore) -> ArgoResult<()> {
    let rows = sqlx::query("PRAGMA table_info(worker)")
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("worker table info: {e}")))?;
    let columns: Vec<String> = rows.into_iter().map(|row| row.get("name")).collect();
    if !columns.iter().any(|c| c == "fs_root") {
        for stmt in FS_ROOT_SCHEMA.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("fs_root migration: {e}")))?;
        }
    }
    Ok(())
}

async fn ensure_role_columns(store: &SessionStore) -> ArgoResult<()> {
    let rows = sqlx::query("PRAGMA table_info(role)")
        .fetch_all(store.pool())
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
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("add role.{name}: {e}")))?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_worker(
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

async fn load_worker(
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

fn parse_duties(specs: &[String]) -> ArgoResult<Vec<DutyOut>> {
    specs
        .iter()
        .map(|spec| {
            let parts: Vec<&str> = spec.splitn(3, "::").collect();
            if parts.len() != 3 {
                return Err(ArgoError::Provider(
                    "duty must be '<cron>::<entry_action>::<description>'".into(),
                ));
            }
            CronExpr::parse(parts[0])
                .map_err(|e| ArgoError::Provider(format!("duty cron '{}': {e}", parts[0])))?;
            Ok(DutyOut {
                cron: parts[0].to_string(),
                entry_action: parts[1].to_string(),
                description: parts[2].to_string(),
            })
        })
        .collect()
}

fn register_cron_duties(
    cwd: &Path,
    tenant_id: TenantId,
    worker_id: &str,
    duties: &[DutyOut],
) -> ArgoResult<()> {
    if duties.is_empty() {
        return Ok(());
    }
    let dir = cwd.join("cron").join("jobs");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ArgoError::Provider(format!("create_dir_all {}: {e}", dir.display())))?;
    for duty in duties {
        let cron = CronExpr::parse(&duty.cron)
            .map_err(|e| ArgoError::Provider(format!("duty cron '{}': {e}", duty.cron)))?;
        let name = format!("worker-{}-{}", short(worker_id), slug(&duty.entry_action));
        let job = CronJob::new(
            tenant_id,
            &name,
            cron,
            format!("Run worker {worker_id} duty {}: {}", duty.entry_action, duty.description),
            CronTarget {
                platform: "worker".into(),
                chat: worker_id.to_string(),
            },
        );
        let body = serde_json::to_string_pretty(&job)
            .map_err(|e| ArgoError::Provider(format!("serialize cron job: {e}")))?;
        let path = dir.join(format!("{name}.json"));
        std::fs::write(&path, body)
            .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))?;
    }
    Ok(())
}

fn print_worker(worker: &WorkerOut, json: bool) -> ArgoResult<()> {
    if json {
        return print_json(worker);
    }
    println!("worker_id      {}", worker.id);
    println!("tenant_id      {}", worker.tenant_id);
    println!("vertical_id    {}", worker.vertical_id);
    println!("persona        {}", worker.persona.name);
    println!("voice          {}", worker.persona.voice_preamble);
    println!("charter        {}", worker.role.charter);
    if let Some(ref fs_root) = worker.fs_root {
        println!("fs_root        {}", fs_root);
    }
    if !worker.role.duties.is_empty() {
        println!("duties");
        for duty in &worker.role.duties {
            println!("  {} -> {} ({})", duty.cron, duty.entry_action, duty.description);
        }
    }
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize json: {e}")))?;
    println!("{body}");
    Ok(())
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

fn short(id: &str) -> String {
    if id.len() > 24 {
        id[..23].to_string()
    } else {
        id.to_string()
    }
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}
