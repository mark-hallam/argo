//! Vertical template pack loading, caching, and dev-link overrides.
//!
//! The crate is intentionally independent of `argo-vertical`: it manages
//! pack distribution and resolves a pack root on disk. Later Track M briefs
//! consume that root to install the named vertical templates.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod dev_link;
pub mod fetcher;
pub mod loader;
pub mod manifest;

pub use dev_link::DevLinkStore;
pub use fetcher::{FetchRequest, GitHubReleaseFetcher, PackFetcher};
pub use loader::{InstalledPack, PackLoader, PackLoaderConfig};
pub use manifest::{FetcherConfig, PackManifest, PackMetadata, VerticalPackEntry};
