//! Narrow read surface that resolves a worker's `trust_grade` for the
//! M6 skill-availability filter ([`SkillStore::available_for_worker`]).
//!
//! `argo-skill` does NOT depend on `argo-identity` (the dep arrow runs
//! `argo-identity → argo-store`; adding `argo-skill → argo-identity`
//! would invert that). The trait keeps the skill crate ignorant of how
//! a worker row is fetched; the concrete adapter for `IdentityStore`
//! lives in `argo-cli` where both crates are already in scope.
//!
//! ```
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use argo_skill::WorkerTrustReader;
//! use argo_types::{ArgoResult, TenantId};
//! use async_trait::async_trait;
//! use uuid::Uuid;
//!
//! struct AlwaysTwo;
//! #[async_trait]
//! impl WorkerTrustReader for AlwaysTwo {
//!     async fn trust_grade(
//!         &self,
//!         _tenant_id: TenantId,
//!         _worker_id: Uuid,
//!     ) -> ArgoResult<Option<u8>> {
//!         Ok(Some(2))
//!     }
//! }
//! let reader = AlwaysTwo;
//! let grade = reader.trust_grade(TenantId::new(), Uuid::nil()).await?;
//! assert_eq!(grade, Some(2));
//! # Ok(()) }
//! ```

use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use uuid::Uuid;

/// Reader returning a worker's numeric trust grade on the same
/// ladder as `skill.min_trust_grade`. `Ok(None)` means "no row for
/// that (tenant, worker) pair"; callers turn that into an explicit
/// error rather than a silent default so a typo / wrong tenant
/// surfaces loudly.
#[async_trait]
pub trait WorkerTrustReader: Send + Sync {
    /// Look up the worker's `trust_grade`. Tenant-scoped so the
    /// caller cannot accidentally bleed across tenants. Returns
    /// `Ok(None)` when no matching row exists; `Ok(Some(grade))`
    /// otherwise. `grade == 0` represents the legacy / fresh-row
    /// default (`worker.trust_grade INTEGER NOT NULL DEFAULT 0`,
    /// migration `50__worker_trust_grade.sql`) and the
    /// most-restrictive end of the ladder.
    async fn trust_grade(
        &self,
        tenant_id: TenantId,
        worker_id: Uuid,
    ) -> ArgoResult<Option<u8>>;
}
