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
//!
//! ## Module layout
//!
//! Per `codex-loc-budget-followup-wave3` (HARD RULE 6 — parent must
//! stay under 500 LOC), the CRUD functions are split:
//! - Read-path (`get`, `get_head`, `list_versions`) lives in
//!   `workflow_definition_read.rs`.
//! - Write-path (`create`, `set_head`, `archive`) lives in
//!   `workflow_definition_write.rs`.
//! - This parent holds the types only and re-exports the functions
//!   so the external path `workflow_definition::*` is unchanged.

use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[path = "workflow_definition_read.rs"]
mod workflow_definition_read;
#[path = "workflow_definition_write.rs"]
mod workflow_definition_write;

pub use workflow_definition_read::{get, get_head, list_versions};
pub use workflow_definition_write::{
    archive, archive_in_tx, create, rollback_to, rollback_to_in_tx, set_head, set_head_in_tx,
};

/// Error type for [`create`]. Wraps [`ArgoError`] for the general case
/// and carries a typed [`Self::UniqueViolation`] variant so the gateway
/// can map a `(tenant, vertical, name, version)` UNIQUE-constraint race
/// to `409 Conflict` with the colliding triple in the body, instead of
/// the generic `500 Internal Server Error` that a stringly-typed
/// `ArgoError::Provider` would surface.
///
/// Lossy `From<WorkflowDefinitionError> for ArgoError` is intentionally
/// not provided — callers that need the typed branch must match
/// directly. The handler in
/// `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs`
/// is the sole production consumer.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowDefinitionError {
    /// Concurrent inserts on the same `(tenant_id, vertical_id, name)`
    /// triple both computed the same `MAX(version) + 1` and the loser
    /// of the race tripped the SQLite UNIQUE constraint. The fields
    /// echo back the colliding namespace so the gateway can render a
    /// useful 409 body.
    #[error("workflow already exists: vertical={vertical}, name={name}")]
    UniqueViolation {
        /// Vertical id that was being written.
        vertical: String,
        /// Workflow name that was being written.
        name: String,
    },
    /// Any other error path — schema validation, sqlx I/O, invariant
    /// breach — pre-existing call sites in this crate still surface
    /// through this transparent wrapper.
    #[error(transparent)]
    Argo(#[from] ArgoError),
}

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
pub enum WorkflowLifecycleState {
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

impl WorkflowLifecycleState {
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
    /// Lifecycle state — see [`WorkflowLifecycleState`].
    pub lifecycle_state: WorkflowLifecycleState,
    /// RFC3339 / ISO-8601 timestamp the row was inserted.
    pub created_at: String,
    /// Opaque principal-serialised string; the canonical encoding is
    /// the caller's responsibility (the brief defers Principal
    /// canonicalisation to P1 #6).
    pub modified_by: String,
}

#[cfg(test)]
#[path = "workflow_definition_tests.rs"]
mod workflow_definition_tests;

#[cfg(test)]
#[path = "workflow_definition_rollback_tests.rs"]
mod workflow_definition_rollback_tests;

#[cfg(test)]
#[path = "workflow_definition_tx_rollback_tests.rs"]
mod workflow_definition_tx_rollback_tests;
