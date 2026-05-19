//! Workflow definition persistence — version chain + head pointer.
//!
//! Today workflow definitions live as TOML files under
//! `verticals/<id>/workflows/*.workflow.toml`, loaded once at
//! session bootstrap and frozen for the session's lifetime (see
//! `crates/argo-vertical/src/loader.rs`). This module is the
//! database-side substrate for the upcoming edit / rollback / diff
//! surfaces (Streams P1 #6, P1 #7, P2 #9, P2 #10). File loads remain
//! authoritative for v1; this layer is additive until the edit API
//! (P1 #6) flips the loader to prefer DB rows over filesystem.
//!
//! ## Tables
//!
//! * `workflow_definition` — one row per workflow VERSION. New rows
//!   land via [`create`]; the `parent_id` column threads a chain so
//!   the diff view can walk back to any prior version without a
//!   separate history table. `lifecycle_state` is `draft` / `active`
//!   / `archived` and is validated at this layer (no CHECK constraint
//!   so the migration stays additive — SQLite cannot drop a CHECK
//!   without a table rebuild).
//! * `workflow_definition_index` — per-`(tenant_id, vertical_id, name)`
//!   "currently active" pointer. Separate table (not a column on
//!   `workflow_definition`) so the head can be cleared or swapped
//!   without rewriting any version row.
//!
//! ## CRUD surface
//!
//! Six free-standing async functions mirroring the established
//! `skill_rating` / `feedback` / `persona_revision` shape — each
//! takes a `&Pool<Sqlite>` so callers stay away from the
//! `SessionStore::ensure_workspace` pattern when they already have a
//! pool. The functions:
//!
//! * [`create`] — inserts a new version, computing
//!   `MAX(version) + 1` per `(tenant, vertical, name)`. Parses the
//!   TOML via [`argo_vertical::parse_workflow`] before touching the
//!   DB; malformed bodies bubble up as [`ArgoError::Schema`].
//! * [`get`] — fetch one row by [`WorkflowDefinitionId`].
//! * [`get_head`] — fetch the currently-active row for a
//!   `(tenant, vertical, name)` triple, or `None` if no head is set.
//! * [`list_versions`] — every version row for a triple, newest
//!   first.
//! * [`set_head`] — atomically point the head at a definition id;
//!   returns [`ArgoError::NotFound`] if the id does not exist.
//! * [`archive`] — flip `lifecycle_state` to `archived` and, if the
//!   row was the current head, clear the head pointer in the same
//!   transaction.

use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// Strongly-typed handle for a single `workflow_definition` row.
/// Wraps a UUID so the stored TEXT id is opaque to callers; the
/// underlying string form is reachable via [`Self::as_str`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowDefinitionId(pub String);

impl WorkflowDefinitionId {
    /// Fresh id, generated as a v7 UUID and stringified.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// View as a borrowed `&str` for sqlx bindings.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkflowDefinitionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkflowDefinitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lifecycle state for a workflow definition row. Strings on the
/// SQLite side ("draft" / "active" / "archived"); the CRUD layer
/// validates inbound values and rejects unknown variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Default for newly-created rows. Not promoted to head yet.
    Draft,
    /// Promoted to head via [`set_head`]. Exactly one `active` row
    /// per `(tenant, vertical, name)` is possible only by convention
    /// — the index table enforces head uniqueness, this column does
    /// not.
    Active,
    /// Retired by [`archive`]; downstream loaders skip these.
    Archived,
}

impl LifecycleState {
    /// String form persisted in sqlite.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    /// Parse the stored sqlite string back into a typed state.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            other => Err(ArgoError::Provider(format!(
                "workflow_definition: unknown lifecycle_state '{other}'"
            ))),
        }
    }
}

/// One full row from `workflow_definition`. Returned by [`get`],
/// [`get_head`], and [`list_versions`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Stable row id.
    pub id: WorkflowDefinitionId,
    /// Tenant that owns this definition row.
    pub tenant_id: TenantId,
    /// Vertical id this workflow belongs to (e.g. `pr_checker`).
    pub vertical_id: String,
    /// Workflow name — unique per `(tenant, vertical)`, versioned.
    pub name: String,
    /// Full TOML body, copy-paste portable.
    pub definition_toml: String,
    /// Monotonic version, starts at 1 and increments per `(tenant,
    /// vertical, name)`.
    pub version: i64,
    /// Predecessor row id, if this version was branched from an
    /// earlier one. `None` for the initial version.
    pub parent_id: Option<WorkflowDefinitionId>,
    /// Lifecycle state — see [`LifecycleState`].
    pub lifecycle_state: LifecycleState,
    /// RFC3339 / ISO-8601 timestamp the row was inserted.
    pub created_at: String,
    /// Opaque principal-serialised string; the canonical encoding is
    /// the caller's responsibility (the brief defers Principal
    /// canonicalisation to P1 #6).
    pub modified_by: String,
}

/// Insert a new version row. Computes `MAX(version) + 1` per
/// `(tenant_id, vertical_id, name)` so callers do not have to pass
/// the next version themselves. The TOML body is parsed via
/// [`argo_vertical::parse_workflow`] *before* the DB insert;
/// malformed bodies surface as [`ArgoError::Schema`] and nothing is
/// written.
///
/// Newly-created rows land with `lifecycle_state = Draft`. Use
/// [`set_head`] separately to promote a draft to head.
pub async fn create(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
    definition_toml: &str,
    parent_id: Option<WorkflowDefinitionId>,
    modified_by: &str,
) -> ArgoResult<WorkflowDefinitionId> {
    if vertical_id.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_definition: vertical_id must not be empty".into(),
        ));
    }
    if name.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_definition: name must not be empty".into(),
        ));
    }
    if modified_by.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_definition: modified_by must not be empty".into(),
        ));
    }
    argo_vertical::parse_workflow(definition_toml)?;

    let tenant_str = tenant_id.to_string();
    let next_version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) + 1 FROM workflow_definition \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ?",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition next-version: {e}")))?;

    let id = WorkflowDefinitionId::new();
    let created_at = chrono::Utc::now().to_rfc3339();
    let parent_id_str: Option<String> = parent_id.as_ref().map(|p| p.0.clone());
    sqlx::query(
        "INSERT INTO workflow_definition \
            (id, tenant_id, vertical_id, name, definition_toml, version, \
             parent_id, lifecycle_state, created_at, modified_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_str())
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(definition_toml)
    .bind(next_version)
    .bind(parent_id_str)
    .bind(LifecycleState::Draft.as_str())
    .bind(&created_at)
    .bind(modified_by)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition insert: {e}")))?;
    Ok(id)
}

/// Fetch one row by id; `None` when the row does not exist.
pub async fn get(
    pool: &Pool<Sqlite>,
    id: &WorkflowDefinitionId,
) -> ArgoResult<Option<WorkflowDefinition>> {
    let row = sqlx::query(
        "SELECT id, tenant_id, vertical_id, name, definition_toml, version, \
                parent_id, lifecycle_state, created_at, modified_by \
         FROM workflow_definition WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition get: {e}")))?;
    row.map(decode_row).transpose()
}

/// Fetch the currently-active row for `(tenant, vertical, name)`,
/// joining through `workflow_definition_index`. Returns `None` when
/// no head is set or the head row has been deleted underneath it
/// (the latter is defensive — `archive` clears the head atomically).
pub async fn get_head(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
) -> ArgoResult<Option<WorkflowDefinition>> {
    let tenant_str = tenant_id.to_string();
    let row = sqlx::query(
        "SELECT wd.id, wd.tenant_id, wd.vertical_id, wd.name, wd.definition_toml, \
                wd.version, wd.parent_id, wd.lifecycle_state, wd.created_at, \
                wd.modified_by \
         FROM workflow_definition_index idx \
         JOIN workflow_definition wd ON wd.id = idx.head_id \
         WHERE idx.tenant_id = ? AND idx.vertical_id = ? AND idx.name = ?",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition get_head: {e}")))?;
    row.map(decode_row).transpose()
}

/// Every version row for `(tenant, vertical, name)`, newest first
/// (ORDER BY version DESC). Empty `Vec` when no rows exist.
pub async fn list_versions(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
) -> ArgoResult<Vec<WorkflowDefinition>> {
    let tenant_str = tenant_id.to_string();
    let rows = sqlx::query(
        "SELECT id, tenant_id, vertical_id, name, definition_toml, version, \
                parent_id, lifecycle_state, created_at, modified_by \
         FROM workflow_definition \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ? \
         ORDER BY version DESC",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition list_versions: {e}")))?;
    rows.into_iter().map(decode_row).collect()
}

/// Promote `id` to head for its `(tenant, vertical, name)` triple.
/// The brief specifies the triple in the signature so callers can
/// verify the row belongs to the namespace they think it does
/// before flipping the head. Mismatch (row exists but its
/// `(tenant, vertical, name)` differs) returns
/// [`ArgoError::Invariant`]; a missing id returns
/// [`ArgoError::NotFound`].
///
/// Also flips the row's `lifecycle_state` from `Draft` to `Active`
/// atomically inside the same transaction so the head + state stay
/// consistent.
pub async fn set_head(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
    id: &WorkflowDefinitionId,
) -> ArgoResult<()> {
    let tenant_str = tenant_id.to_string();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head tx: {e}")))?;

    let row = sqlx::query(
        "SELECT tenant_id, vertical_id, name FROM workflow_definition WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head probe: {e}")))?;
    let row = row.ok_or_else(|| {
        ArgoError::NotFound(format!("workflow_definition: id '{}' does not exist", id.0))
    })?;
    let row_tenant: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode tenant_id: {e}")))?;
    let row_vertical: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode vertical_id: {e}")))?;
    let row_name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode name: {e}")))?;
    if row_tenant != tenant_str || row_vertical != vertical_id || row_name != name {
        return Err(ArgoError::Invariant(format!(
            "workflow_definition set_head: id '{}' belongs to \
             ({row_tenant}, {row_vertical}, {row_name}), not ({tenant_str}, {vertical_id}, {name})",
            id.0
        )));
    }

    sqlx::query(
        "INSERT INTO workflow_definition_index (tenant_id, vertical_id, name, head_id) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(tenant_id, vertical_id, name) DO UPDATE SET head_id = excluded.head_id",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(id.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head write: {e}")))?;

    sqlx::query("UPDATE workflow_definition SET lifecycle_state = ? WHERE id = ?")
        .bind(LifecycleState::Active.as_str())
        .bind(id.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head state: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head commit: {e}")))?;
    Ok(())
}

/// Flip `lifecycle_state` to `Archived`. If the row was the head
/// for its `(tenant, vertical, name)` triple, the head pointer is
/// cleared in the same transaction so downstream loaders do not
/// resolve to an archived row. A missing id is a no-op (idempotent —
/// archive of an already-archived or never-existed id is fine).
pub async fn archive(pool: &Pool<Sqlite>, id: &WorkflowDefinitionId) -> ArgoResult<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition archive tx: {e}")))?;

    let probe = sqlx::query(
        "SELECT tenant_id, vertical_id, name FROM workflow_definition WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition archive probe: {e}")))?;
    let Some(row) = probe else {
        // Idempotent: archive of a missing id is a no-op.
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("workflow_definition archive commit: {e}")))?;
        return Ok(());
    };
    let row_tenant: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode tenant_id: {e}")))?;
    let row_vertical: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode vertical_id: {e}")))?;
    let row_name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode name: {e}")))?;

    sqlx::query("UPDATE workflow_definition SET lifecycle_state = ? WHERE id = ?")
        .bind(LifecycleState::Archived.as_str())
        .bind(id.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition archive update: {e}")))?;

    sqlx::query(
        "DELETE FROM workflow_definition_index \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ? AND head_id = ?",
    )
    .bind(&row_tenant)
    .bind(&row_vertical)
    .bind(&row_name)
    .bind(id.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition archive index: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition archive commit: {e}")))?;
    Ok(())
}

fn decode_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkflowDefinition> {
    let id: String = row
        .try_get("id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode id: {e}")))?;
    let tenant_str: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode tenant_id: {e}")))?;
    let tenant_uuid = Uuid::parse_str(&tenant_str).map_err(|e| {
        ArgoError::Provider(format!(
            "workflow_definition decode tenant_id '{tenant_str}': {e}"
        ))
    })?;
    let vertical_id: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode vertical_id: {e}")))?;
    let name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode name: {e}")))?;
    let definition_toml: String = row.try_get("definition_toml").map_err(|e| {
        ArgoError::Provider(format!("workflow_definition decode definition_toml: {e}"))
    })?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode version: {e}")))?;
    let parent_id: Option<String> = row
        .try_get::<Option<String>, _>("parent_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode parent_id: {e}")))?
        .filter(|s| !s.is_empty());
    let lifecycle_state_str: String = row.try_get("lifecycle_state").map_err(|e| {
        ArgoError::Provider(format!("workflow_definition decode lifecycle_state: {e}"))
    })?;
    let created_at: String = row
        .try_get("created_at")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode created_at: {e}")))?;
    let modified_by: String = row
        .try_get("modified_by")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode modified_by: {e}")))?;
    Ok(WorkflowDefinition {
        id: WorkflowDefinitionId(id),
        tenant_id: TenantId(tenant_uuid),
        vertical_id,
        name,
        definition_toml,
        version,
        parent_id: parent_id.map(WorkflowDefinitionId),
        lifecycle_state: LifecycleState::parse(&lifecycle_state_str)?,
        created_at,
        modified_by,
    })
}

#[cfg(test)]
#[path = "workflow_definition_tests.rs"]
mod workflow_definition_tests;
