//! Demo orchestration sessions for the canonical v1.0 seed.
//!
//! Closes UX audit `whole-site F08` — `/orchestration` rendered "No
//! orchestration yet" because the seed wrote no `session` rows with
//! `provider = 'orchestrator'`. The gateway endpoint
//! `GET /api/v1/orchestration` (see
//! `crates/argo-cli/src/handlers/gateway/routes_orchestration.rs`)
//! selects directly against `session WHERE provider = 'orchestrator'`,
//! so seeding two orchestrator-shaped session rows is the smallest fix
//! that makes the page render.
//!
//! Pattern (workspace `INSERT OR IGNORE` + session insert with the
//! enriched `status`/`participants`/`created_at`/`deadline` columns from
//! migration v16) mirrors `argo_orchestrator::persist::insert_session`.
//! Re-implementing the SQL inline here (rather than adding an
//! `argo-orchestrator` dependency) keeps the demo-seed crate's dep
//! footprint small — the same trade-off the existing `workflow.rs`
//! sibling makes for `workflow_definition`.

use argo_identity::IdentityStore;
use argo_store::{SessionStore, WorkerId};
use argo_types::{ArgoError, TenantId};
use chrono::{Duration as ChronoDuration, Utc};
use uuid::Uuid;

use crate::SeedResult;

/// Provider literal the gateway selects for the orchestration index.
/// Matches `argo_orchestrator::persist::ORCHESTRATOR_PROVIDER`.
const ORCHESTRATOR_PROVIDER: &str = "orchestrator";

/// One demo orchestration session entry.
struct OrchEntry {
    persona_name: &'static str,
    title: &'static str,
    deadline_hours: i64,
}

/// Insert the 2 canonical demo orchestration sessions under `tenant_id`.
/// Returns the number of session rows newly inserted. If a required
/// persona is missing from the tenant, that entry is skipped (the
/// function does not fail closed — same pattern as [`crate::retros`]).
///
/// # Errors
///
/// Propagates any identity-store or SQLite execution error.
pub(crate) async fn seed_demo_orchestration(
    store: &SessionStore,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;
    let workers = identities.list_workers(tenant_id, false).await?;

    let resolve = |name: &str| -> Option<WorkerId> {
        workers
            .iter()
            .find(|w| w.persona.name == name)
            .map(|w| w.id)
    };

    let entries: [OrchEntry; 2] = [
        OrchEntry {
            persona_name: "Mariah",
            title: "Marketplace listing → re-price weekly",
            deadline_hours: 24 * 7,
        },
        OrchEntry {
            persona_name: "Peta",
            title: "New customer onboarding → handoff to project-coordination",
            deadline_hours: 24 * 3,
        },
    ];

    ensure_session_columns(store).await?;
    ensure_orchestrator_workspace(store, tenant_id).await?;

    let pool = store.pool();
    let tenant_blob = *tenant_id.0.as_bytes();
    let now = Utc::now();
    let now_micros = now.timestamp_micros();
    let mut inserted = 0;

    for entry in &entries {
        let Some(worker_id) = resolve(entry.persona_name) else {
            tracing::warn!(
                persona = entry.persona_name,
                "demo orchestration session skipped — persona not found"
            );
            continue;
        };
        let session_id = *Uuid::now_v7().as_bytes();
        let deadline = (now + ChronoDuration::hours(entry.deadline_hours)).timestamp_micros();
        let participants = serde_json::to_string(&[worker_id.to_string()]).map_err(|e| {
            crate::SeedError::Argo(ArgoError::Provider(format!(
                "orchestration participants: {e}"
            )))
        })?;
        sqlx::query(
            "INSERT INTO session (id, tenant_id, workspace_id, title, started_at, \
             model, provider, status, participants, created_at, deadline) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session_id.as_slice())
        .bind(tenant_blob.as_slice())
        .bind(tenant_blob.as_slice())
        .bind(entry.title)
        .bind(now_micros)
        .bind(ORCHESTRATOR_PROVIDER)
        .bind(ORCHESTRATOR_PROVIDER)
        .bind("active")
        .bind(&participants)
        .bind(now_micros)
        .bind(deadline)
        .execute(pool)
        .await
        .map_err(|e| {
            crate::SeedError::Argo(ArgoError::Provider(format!(
                "insert orchestration session: {e}"
            )))
        })?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Workspace row used by the orchestrator-provider session inserts.
/// Mirrors `argo_orchestrator::persist::insert_session`'s
/// `INSERT OR IGNORE INTO workspace` — we reuse the tenant blob as the
/// workspace id so the seed does not need to look up an existing one.
async fn ensure_orchestrator_workspace(
    store: &SessionStore,
    tenant_id: TenantId,
) -> SeedResult<()> {
    let pool = store.pool();
    let tenant_blob = *tenant_id.0.as_bytes();
    let created_at = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT OR IGNORE INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant_blob.as_slice())
    .bind(tenant_blob.as_slice())
    .bind(ORCHESTRATOR_PROVIDER)
    .bind("")
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|e| {
        crate::SeedError::Argo(ArgoError::Provider(format!(
            "orchestrator workspace: {e}"
        )))
    })?;
    Ok(())
}

/// Ensure the additive `status` / `participants` / `created_at` /
/// `deadline` columns from migration v16 are present on the `session`
/// table. The orchestrator's `ensure_schema` does the same lazily; we
/// repeat the check here so a `seed_demo_orchestration` call
/// against a freshly-migrated store does not race the orchestrator's
/// first session insert (the orchestrator may not have run yet under
/// a CLI-only path).
async fn ensure_session_columns(store: &SessionStore) -> SeedResult<()> {
    let pool = store.pool();
    add_column_if_missing(pool, "session", "status", "TEXT NOT NULL DEFAULT 'active'").await?;
    add_column_if_missing(
        pool,
        "session",
        "participants",
        "TEXT NOT NULL DEFAULT '[]'",
    )
    .await?;
    add_column_if_missing(pool, "session", "created_at", "INTEGER NOT NULL DEFAULT 0").await?;
    add_column_if_missing(pool, "session", "deadline", "INTEGER").await?;
    Ok(())
}

async fn add_column_if_missing(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    table: &str,
    column: &str,
    decl: &str,
) -> SeedResult<()> {
    use sqlx::Row;
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await
        .map_err(|e| {
            crate::SeedError::Argo(ArgoError::Provider(format!(
                "PRAGMA table_info({table}): {e}"
            )))
        })?;
    let already_has = rows.iter().any(|r| {
        r.try_get::<String, _>("name")
            .map(|n| n == column)
            .unwrap_or(false)
    });
    if already_has {
        return Ok(());
    }
    sqlx::query(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"))
        .execute(pool)
        .await
        .map_err(|e| {
            crate::SeedError::Argo(ArgoError::Provider(format!(
                "ALTER TABLE {table} ADD COLUMN {column}: {e}"
            )))
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personas::DEMO_SEED_PERSONAS;
    use argo_identity::{Persona, Role, Worker};
    use sqlx::Row;
    use std::sync::Arc;

    async fn open_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    async fn seed_canonical_workers(store: &SessionStore, tenant: TenantId) {
        let identities = IdentityStore::new(store);
        identities.ensure_schema().await.unwrap();
        for spine in DEMO_SEED_PERSONAS {
            let persona = Persona::named(spine.name, "test");
            let role = Role::new("test");
            let worker = Worker::draft(tenant, spine.vertical_id, persona, role);
            identities.create_worker(&worker).await.unwrap();
        }
    }

    #[tokio::test]
    async fn seeds_two_orchestrator_sessions() {
        let store = open_store().await;
        let tenant = TenantId::new();
        seed_canonical_workers(&store, tenant).await;

        let inserted = seed_demo_orchestration(&store, tenant).await.unwrap();
        assert_eq!(inserted, 2);

        let rows = sqlx::query(
            "SELECT title, provider, status FROM session \
             WHERE tenant_id = ? AND provider = 'orchestrator' \
             ORDER BY started_at DESC",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(store.pool())
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        let titles: Vec<String> = rows
            .iter()
            .map(|r| r.try_get::<String, _>("title").unwrap())
            .collect();
        assert!(
            titles.iter().any(|t| t.contains("Marketplace listing")),
            "Mariah orchestration session missing; got {titles:?}"
        );
        assert!(
            titles.iter().any(|t| t.contains("project-coordination")),
            "Peta orchestration session missing; got {titles:?}"
        );
        for r in &rows {
            assert_eq!(r.try_get::<String, _>("provider").unwrap(), "orchestrator");
            assert_eq!(r.try_get::<String, _>("status").unwrap(), "active");
        }
    }

    #[tokio::test]
    async fn sibling_tenant_sees_zero_orchestration_rows() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let other = TenantId::new();
        seed_canonical_workers(&store, tenant).await;
        seed_demo_orchestration(&store, tenant).await.unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM session \
             WHERE tenant_id = ? AND provider = 'orchestrator'",
        )
        .bind(other.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert_eq!(count, 0);
    }
}
