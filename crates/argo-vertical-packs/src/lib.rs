//! Vertical template pack loading, caching, and dev-link overrides.
//!
//! Distribution mechanics (fetch / cache / dev-link / resolve) are
//! self-contained here. The crate also exposes a narrow bridge —
//! see [`adapter`] — that translates a cached pack into the shape
//! the runtime `argo_vertical` loader registers, so one pack can
//! be loaded without a parallel `verticals/` entry.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod adapter;
pub mod dev_link;
pub mod fetcher;
pub mod loader;
pub mod manifest;

pub use adapter::adapt_pack_entry;
pub use dev_link::DevLinkStore;
pub use fetcher::{FetchRequest, GitHubReleaseFetcher, PackFetcher};
pub use loader::{InstalledPack, PackLoader, PackLoaderConfig};
pub use manifest::{FetcherConfig, PackManifest, PackMetadata, VerticalPackEntry};
