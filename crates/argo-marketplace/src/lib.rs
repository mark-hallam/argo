//! argo-marketplace — workspace OCI bundle push/pull (§23).
//!
//! Phase 6.5 ships:
//! - **Deterministic tar assembly** ([`build_bundle`]) — the same
//!   workspace state always produces a byte-identical tar + digest,
//!   so the §23.3 "round-trip yields the same digest" property holds.
//! - **OCI registry seam** ([`OciRegistry`]) — trait + three impls:
//!     - [`StubRegistry`] for the no-network / "needs the daemon"
//!       error message.
//!     - [`MemoryRegistry`] for hermetic round-trip tests.
//!     - [`OciDistributionRegistry`] — real HTTP client against the
//!       OCI Distribution Spec v1 (Docker Registry API v2) for
//!       GHCR, Docker Hub, Artifact Registry, Harbor, etc.
//!
//! ```no_run
//! # use std::path::Path;
//! # use argo_marketplace::{build_bundle, OciDistributionRegistry, OciReference, OciRegistry};
//! # async fn demo(workspace_root: &Path, out_tar: &Path) -> Result<(), Box<dyn std::error::Error>> {
//! let manifest = build_bundle(workspace_root, "my-workspace", "0.1", out_tar).await?;
//! let reference = OciReference::parse("ghcr.io/argo-agents/my-workspace:0.1")?;
//! let tar_bytes = tokio::fs::read(out_tar).await?;
//! let registry = OciDistributionRegistry::new("https://ghcr.io", None);
//! registry.push(&reference, &tar_bytes, &manifest).await?;
//! # Ok(())
//! # }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod bundle;
mod bundle_vertical;
mod oci_auth;
mod oci_distribution;
mod registry;

pub use bundle::{build_bundle, manifests_match, BundleEntry, BundleManifest, BUNDLE_PATHS};
pub use bundle_vertical::{build_vertical_bundle, unpack_vertical_bundle};
pub use oci_auth::{resolve_registry_auth, BearerChallenge, ResolvedRegistryAuth};
pub use oci_distribution::{
    OciDescriptor, OciDistributionRegistry, OciImageManifest, CONFIG_MEDIA_TYPE, LAYER_MEDIA_TYPE,
    MANIFEST_MEDIA_TYPE,
};
pub use registry::{MemoryRegistry, OciReference, OciRegistry, StubRegistry};
