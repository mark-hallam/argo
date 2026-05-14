//! SQL plumbing for [`IdentityStore`](crate::store::IdentityStore):
//! schema-ensure helpers, the `worker` SELECT projection, row decoding,
//! and shared serde helpers. Extracted so `store.rs` stays under the
//! 500-LOC ceiling; mirrors prior splits like `app.rs` + `app_navigation.rs`.
//!
//! All items are crate-private — the public API is unchanged.

use crate::{
    CircuitState, Duty, MemoryNs, MemoryScope, Persona, Role, VoiceProfile, Worker, WorkerId,
    WorkerLevel,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

pub(crate) const IDENTITY_SCHEMA: &str =
    include_str!("../../argo-store/migrations/12__worker_identity.sql");
// Documentation-only: ALTER TABLE statements applied via
// `ensure_worker_charter_columns` (PRAGMA-based idempotent path,
// mirroring `ensure_role_columns`).
const _WORKER_CHARTER_SCHEMA: &str =
    include_str!("../../argo-store/migrations/30__worker_charter.sql");
// Documentation-only: ALTER TABLE for the tenure `level` column,
// applied via `ensure_worker_level_columns`. Stream
// `claude-worker-promotion-workflow`.
const _WORKER_LEVEL_SCHEMA: &str =
    include_str!("../../argo-store/migrations/45__worker_level.sql");
// Documentation-only: ALTER TABLE for `trust_grade` + `last_used_at`,
// applied via `ensure_worker_trust_grade_columns`. Stream
// `mvp-worker-trust-grade-column`.
const _WORKER_TRUST_GRADE_SCHEMA: &str =
    include_str!("../../argo-store/migrations/50__worker_trust_grade.sql");
// Raw schema for the worker↔skill association table, applied via
// [`ensure_worker_skill_table`]. Stream `mvp-worker-skills-table`.
const WORKER_SKILL_SCHEMA: &str =
    include_str!("../../argo-store/migrations/51__worker_skill.sql");
// Documentation-only: ALTER TABLE for `role_name`, applied via
// `ensure_worker_role_name_columns`. Stream
// `mission-routing-skillmatch-rolematch`.
const _WORKER_ROLE_NAME_SCHEMA: &str =
    include_str!("../../argo-store/migrations/61__worker_role_name.sql");

pub(crate) const WORKER_SELECT: &str = "SELECT
    w.id, w.tenant_id, w.vertical_id, w.memory_scope,
    w.auto_approval_json, w.channels_json,
    w.charter_md, w.kpi_json, w.level,
    w.trust_grade, w.last_used_at, w.role_name,
    p.name, p.voice_preamble, p.avatar, p.bio, p.builder_attribution,
    p.provenance_chain_json,
    r.charter, r.kpis_json, r.escalation
FROM worker w
JOIN persona p ON p.id = w.persona_id
JOIN role r ON r.worker_id = w.id
WHERE w.id = ? AND w.tenant_id = ?";

pub(crate) async fn ensure_worker_charter_columns(
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> ArgoResult<()> {
    let rows = sqlx::query("PRAGMA table_info(worker)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker table info: {e}")))?;
    let columns: Vec<String> = rows.into_iter().map(|row| row.get("name")).collect();
    for (name, ty) in [("charter_md", "TEXT"), ("kpi_json", "TEXT")] {
        if !columns.iter().any(|existing| existing == name) {
            let sql = format!("ALTER TABLE worker ADD COLUMN {name} {ty}");
            sqlx::query(&sql)
                .execute(pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("add worker.{name}: {e}")))?;
        }
    }
    Ok(())
}

/// Idempotent additive migration for `worker.level` (Stream
/// `claude-worker-promotion-workflow`). Mirrors
/// [`ensure_worker_charter_columns`]: PRAGMA-checks the column,
/// `ALTER TABLE ADD COLUMN ... NOT NULL DEFAULT 'entry'` if missing,
/// and best-effort creates the supporting index.
pub(crate) async fn ensure_worker_level_columns(
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> ArgoResult<()> {
    let rows = sqlx::query("PRAGMA table_info(worker)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker table info: {e}")))?;
    let columns: Vec<String> = rows.into_iter().map(|row| row.get("name")).collect();
    if !columns.iter().any(|existing| existing == "level") {
        sqlx::query("ALTER TABLE worker ADD COLUMN level TEXT NOT NULL DEFAULT 'entry'")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("add worker.level: {e}")))?;
    }
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_worker_level ON worker(level)")
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Io(format!("create idx_worker_level: {e}")))?;
    Ok(())
}

/// Idempotent additive migration for `worker.trust_grade` +
/// `worker.last_used_at` (Stream `mvp-worker-trust-grade-column`).
/// Mirrors [`ensure_worker_charter_columns`] / [`ensure_worker_level_columns`]:
/// PRAGMA-checks each column, `ALTER TABLE ADD COLUMN ...` if missing,
/// best-effort creates the supporting DESC indexes.
pub(crate) async fn ensure_worker_trust_grade_columns(
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> ArgoResult<()> {
    let rows = sqlx::query("PRAGMA table_info(worker)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker table info: {e}")))?;
    let columns: Vec<String> = rows.into_iter().map(|row| row.get("name")).collect();
    if !columns.iter().any(|existing| existing == "trust_grade") {
        sqlx::query("ALTER TABLE worker ADD COLUMN trust_grade INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("add worker.trust_grade: {e}")))?;
    }
    if !columns.iter().any(|existing| existing == "last_used_at") {
        sqlx::query("ALTER TABLE worker ADD COLUMN last_used_at TEXT")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("add worker.last_used_at: {e}")))?;
    }
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_worker_trust_grade \
         ON worker(trust_grade DESC)",
    )
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Io(format!("create idx_worker_trust_grade: {e}")))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_worker_last_used_at \
         ON worker(last_used_at DESC)",
    )
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Io(format!("create idx_worker_last_used_at: {e}")))?;
    Ok(())
}

/// Idempotent additive migration for `worker.role_name` (Stream
/// `mission-routing-skillmatch-rolematch`, Tier M4). PRAGMA-checks the
/// column, `ALTER TABLE ADD COLUMN ... TEXT` if missing, best-effort
/// creates the supporting index. Mirrors
/// [`ensure_worker_trust_grade_columns`].
pub(crate) async fn ensure_worker_role_name_columns(
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> ArgoResult<()> {
    let rows = sqlx::query("PRAGMA table_info(worker)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker table info: {e}")))?;
    let columns: Vec<String> = rows.into_iter().map(|row| row.get("name")).collect();
    if !columns.iter().any(|existing| existing == "role_name") {
        sqlx::query("ALTER TABLE worker ADD COLUMN role_name TEXT")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("add worker.role_name: {e}")))?;
    }
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_worker_role_name ON worker(role_name)")
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Io(format!("create idx_worker_role_name: {e}")))?;
    Ok(())
}

/// Idempotent additive helper for the `worker_skill` table (Stream
/// `mvp-worker-skills-table`). Mirrors [`ensure_worker_level_columns`]:
/// applies each statement from the raw SQL file via
/// `CREATE TABLE/INDEX IF NOT EXISTS`, so the crate-scoped
/// `ensure_schema` path matches the versioned-migration path
/// (`v47.rs`) on fresh DBs.
pub(crate) async fn ensure_worker_skill_table(
    pool: &sqlx::Pool<sqlx::Sqlite>,
) -> ArgoResult<()> {
    for stmt in WORKER_SKILL_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("ensure worker_skill: {e}")))?;
    }
    Ok(())
}

pub(crate) async fn ensure_role_columns(pool: &sqlx::Pool<sqlx::Sqlite>) -> ArgoResult<()> {
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

pub(crate) fn kpis_to_json(kpis: Option<&serde_json::Value>) -> ArgoResult<Option<String>> {
    kpis.map(|v| {
        serde_json::to_string(v)
            .map_err(|e| ArgoError::Provider(format!("serialize kpis: {e}")))
    })
    .transpose()
}

pub(crate) async fn insert_duty(
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

pub(crate) fn worker_from_row(
    row: sqlx::sqlite::SqliteRow,
    duties: Vec<Duty>,
) -> ArgoResult<Worker> {
    let id = parse_worker_id(row.get("id"))?;
    let tenant_id = parse_tenant_id(row.get("tenant_id"))?;
    let scope = MemoryScope::parse(row.get("memory_scope"))
        .ok_or_else(|| ArgoError::Provider("unknown memory scope".into()))?;
    let persona = Persona {
        name: row.get("name"),
        voice_profile: VoiceProfile {
            preamble: row.get("voice_preamble"),
            ..Default::default()
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
    let charter: Option<String> = row.get("charter_md");
    let kpis = match row.get::<Option<String>, _>("kpi_json") {
        Some(raw) => Some(
            serde_json::from_str(&raw)
                .map_err(|e| ArgoError::Provider(format!("parse worker kpi_json: {e}")))?,
        ),
        None => None,
    };
    let level_raw: String = row.get("level");
    let level = WorkerLevel::from_str(&level_raw)
        .map_err(|e| ArgoError::Provider(format!("parse worker.level: {e}")))?;
    let trust_grade_raw: i64 = row.get("trust_grade");
    let trust_grade = u8::try_from(trust_grade_raw)
        .map_err(|e| ArgoError::Provider(format!("parse worker.trust_grade {trust_grade_raw}: {e}")))?;
    let last_used_at = match row.get::<Option<String>, _>("last_used_at") {
        Some(raw) => Some(
            DateTime::parse_from_rfc3339(&raw)
                .map_err(|e| ArgoError::Provider(format!("parse worker.last_used_at: {e}")))?
                .with_timezone(&Utc),
        ),
        None => None,
    };
    let role_name: Option<String> = row.get("role_name");
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
        charter,
        kpis,
        level,
        trust_grade,
        last_used_at,
        role_name,
    })
}

pub(crate) fn to_json<T: Serialize>(value: &T, label: &str) -> ArgoResult<String> {
    serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialize {label}: {e}")))
}

pub(crate) fn from_json<T: serde::de::DeserializeOwned>(value: &str, label: &str) -> ArgoResult<T> {
    serde_json::from_str(value).map_err(|e| ArgoError::Provider(format!("parse {label}: {e}")))
}

pub(crate) fn parse_worker_id(value: &str) -> ArgoResult<WorkerId> {
    value
        .parse()
        .map_err(|e| ArgoError::Provider(format!("worker id '{value}': {e}")))
}

pub(crate) fn parse_tenant_id(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}
