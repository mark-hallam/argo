//! Tenant pre-flight gate for the macOS computer-use executor.
//!
//! The executor exists as a per-call surface that turns a
//! `(tenant_id, worker_id)` request into native macOS automation.
//! `TenantWorkerLookup` is the thin trait the executor uses to
//! verify that the calling tenant owns the worker referenced by
//! the request, BEFORE any macOS API call runs.
//!
//! - Production multi-tenant callers (gateway, agent runner) wrap
//!   an `Arc<SessionStore>` via the blanket impl below; the
//!   executor calls `IdentityStore::get_worker(tenant, worker)`
//!   and refuses with `MacosCuError::TenantMismatch` when the
//!   pair does not exist.
//! - The single-tenant CLI dispatch path uses
//!   `NoopTenantWorkerLookup` — explicit "this path is trusted"
//!   marker so reviewers see the absence of enforcement at the
//!   call site rather than via an implicit default.

use argo_identity::{IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use async_trait::async_trait;
use std::sync::Arc;

/// Verify that a `(tenant_id, worker_id)` pair refers to a
/// real worker the caller tenant owns. Implementations MUST NOT
/// disclose whether the worker exists in another tenant — return
/// `false` for cross-tenant lookups identical to "not found", so
/// the caller cannot distinguish the two cases.
#[async_trait]
pub trait TenantWorkerLookup: Send + Sync {
    /// `true` iff the worker exists AND the caller tenant owns it.
    /// All other cases (worker absent, worker in a different
    /// tenant, lookup error) return `false`.
    async fn worker_owned_by_tenant(&self, tenant_id: TenantId, worker_id: WorkerId) -> bool;
}

/// Canonical multi-tenant data path: a shared `SessionStore`
/// opens an `IdentityStore` per call and asks SQL — which is
/// already tenant-scoped (verified by
/// `sql_primitives_are_tenant_scoped_today` in
/// `tests/cross_tenant_capability_test.rs`).
///
/// Implemented on the bare `SessionStore` rather than
/// `Arc<SessionStore>` so callers can pass either an
/// `Arc<SessionStore>` (most common) or any other shared handle
/// and coerce it into `Arc<dyn TenantWorkerLookup>`.
#[async_trait]
impl TenantWorkerLookup for SessionStore {
    async fn worker_owned_by_tenant(&self, tenant_id: TenantId, worker_id: WorkerId) -> bool {
        let identities = IdentityStore::new(self);
        matches!(identities.get_worker(tenant_id, worker_id).await, Ok(Some(_)))
    }
}

/// Explicit "no enforcement" lookup — only legitimate when the
/// caller is single-tenant by construction (today's CLI dispatch
/// path; the operator's own workspace store). Wired in code so
/// reviewers see the opt-out at the call site instead of an
/// implicit default.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopTenantWorkerLookup;

#[async_trait]
impl TenantWorkerLookup for NoopTenantWorkerLookup {
    async fn worker_owned_by_tenant(&self, _tenant_id: TenantId, _worker_id: WorkerId) -> bool {
        true
    }
}

/// Convenience: build a noop lookup wrapped in `Arc<dyn ...>`.
#[must_use]
pub fn noop_tenant_lookup() -> Arc<dyn TenantWorkerLookup> {
    Arc::new(NoopTenantWorkerLookup)
}
