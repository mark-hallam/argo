//! argo-marketplace — workspace OCI bundle push/pull (§23).
//!
//! Phase 6.5 ships:
//! - **Deterministic tar assembly** ([`build_bundle`]) — the same
//!   workspace state always produces a byte-identical tar + digest,
//!   so the §23.3 "round-trip yields the same digest" property holds.
//! - **OCI registry seam** ([`OciRegistry`]) — trait + a clean stub
//!   ([`StubRegistry`]). The real `oci-distribution` push/pull lands
//!   with the §23.3 integration-test PR (against test accounts on
//!   GHCR / Docker Hub / Artifact Registry).
//!
//! ```no_run
//! # use std::path::Path;
//! # use argo_marketplace::{build_bundle, OciReference, OciRegistry, StubRegistry};
//! # async fn demo(workspace_root: &Path, out_tar: &Path) -> Result<(), Box<dyn std::error::Error>> {
//! let manifest = build_bundle(workspace_root, "my-workspace", "0.1", out_tar).await?;
//! let reference = OciReference::parse("ghcr.io/argo-agents/my-workspace:0.1")?;
//! let tar_bytes = tokio::fs::read(out_tar).await?;
//! let registry = StubRegistry; // real client lands in §23.3.
//! let _ = registry.push(&reference, &tar_bytes, &manifest).await; // returns Provider error
//! # Ok(())
//! # }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod bundle;
mod registry;

pub use bundle::{build_bundle, manifests_match, BundleEntry, BundleManifest, BUNDLE_PATHS};
pub use registry::{MemoryRegistry, OciReference, OciRegistry, StubRegistry};
