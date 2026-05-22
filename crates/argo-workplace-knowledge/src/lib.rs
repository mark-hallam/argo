//! Tenant-scoped workplace-knowledge wiki substrate
//! (Stream `mission-workplace-knowledge-substrate`).
//!
//! Two tables — `workplace_knowledge_doc` (one head row per wiki
//! page, keyed `UNIQUE(tenant_id, slug)`) and
//! `workplace_knowledge_revision` (append-only edit history) — back
//! the operator-authored handbook that downstream Wave-A briefs
//! read from / write to / inject into agent system messages.
//!
//! Public surface:
//!
//! * [`doc::create`] / [`doc::update`] / [`doc::delete`] /
//!   [`doc::get`] / [`doc::list`] — head CRUD; `update` transactionally
//!   bumps `version` and writes the prior body to the revision log.
//! * [`doc::max_version_for_tenant`] — cheap cache-key probe used by
//!   the context-injection hot path (no row materialisation).
//! * [`revision::list_for_doc`] / [`revision::get`] — read access to
//!   the historical edits for a single doc.
//!
//! Cross-tenant reads/writes are guarded at every call site by
//! binding `tenant_id` into the WHERE clause; the `(tenant_id, slug)`
//! UNIQUE index plus the `tenant_id` predicate on every SELECT mean
//! tenant A cannot see, mutate, or shadow tenant B's docs.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::SessionStore;
//! use argo_types::TenantId;
//! use argo_workplace_knowledge::doc::{self, KnowledgeCategory};
//!
//! let store = SessionStore::open(":memory:").await
//!     .map_err(|e| argo_types::ArgoError::Provider(e.to_string()))?;
//! store.migrate().await
//!     .map_err(|e| argo_types::ArgoError::Provider(e.to_string()))?;
//! let tenant = TenantId::new();
//! let head = doc::create(
//!     store.pool(),
//!     &tenant,
//!     "refund-policy",
//!     "Refund policy",
//!     KnowledgeCategory::BusinessRules,
//!     "Refunds are issued within 30 days …",
//!     "operator@example.com",
//! ).await?;
//! assert_eq!(head.version, 1);
//! # Ok(()) }
//! ```

pub mod doc;
pub mod revision;

pub use doc::{KnowledgeCategory, WorkplaceKnowledgeDoc};
pub use revision::WorkplaceKnowledgeRevision;
